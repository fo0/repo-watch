//! The serialized request queue: ETag-cached pagination, rate-limit backoff and the
//! exactly-once dispatch call.
use super::{
    parse_dispatch_confirmation, ApiError, ApiRequest, ApiResponse, ApiUrl, Method, Transport,
    TransportError, MAX_BACKOFF_SECONDS, MAX_PAGES, MAX_RESPONSE_BYTES,
};
use crate::i18n::tr;
use crate::models::{DispatchOutcome, DispatchState};
use crate::trf;
use crate::workflow::looks_like_credential;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

#[derive(Clone)]
pub(super) struct CachedPage {
    etag: Option<String>,
    body: Vec<u8>,
    next: Option<ApiUrl>,
}

/// Bodies of one pagination traversal; `truncated` marks a prefix cut at the page budget.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pages {
    pub bodies: Vec<Vec<u8>>,
    pub truncated: bool,
}

/// The bodies of a listing that fit the page budget, or `Truncated`.
fn complete(pages: Pages) -> Result<Vec<Vec<u8>>, ApiError> {
    if pages.truncated {
        return Err(ApiError::Truncated);
    }
    Ok(pages.bodies)
}

pub(super) struct ClientState<T> {
    transport: T,
    pub(super) pages: BTreeMap<(String, ApiUrl), CachedPage>,
    backoff_until: BTreeMap<String, u64>,
    rate_limit_failures: BTreeMap<String, u32>,
}

/// All operations, including a complete pagination traversal, share one serialized queue.
/// Backoff returns a deadline; it never sleeps a UI thread or automatically retries a POST.
pub struct ApiClient<T: Transport> {
    pub(super) state: Mutex<ClientState<T>>,
}

impl<T: Transport> ApiClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            state: Mutex::new(ClientState {
                transport,
                pages: BTreeMap::new(),
                backoff_until: BTreeMap::new(),
                rate_limit_failures: BTreeMap::new(),
            }),
        }
    }

    pub fn into_transport(self) -> Result<T, ApiError> {
        self.state
            .into_inner()
            .map(|state| state.transport)
            .map_err(|_| ApiError::QueueUnavailable)
    }

    pub fn execute(&self, request: &ApiRequest, now: u64) -> Result<ApiResponse, ApiError> {
        let mut state = self.state.lock().map_err(|_| ApiError::QueueUnavailable)?;
        send_once(&mut state, request, now)
    }

    /// A GET whose throttling answer (a 403/429 that armed a backoff) surfaces as `RateLimited`,
    /// so callers never mistake a rate limit for missing access. Other statuses stay `Ok`.
    pub fn get(&self, request: &ApiRequest, now: u64) -> Result<ApiResponse, ApiError> {
        if request.method != Method::Get {
            return Err(ApiError::Policy(tr(
                "Nur GET-Anfragen sind hier erlaubt.",
                "Only GET requests are allowed here.",
            )));
        }
        self.send_unthrottled(request, now)
    }

    /// A POST sent exactly once and never retried. Like `get`, a throttling answer surfaces
    /// as `RateLimited`; every other status stays `Ok` for the caller to interpret.
    pub fn post(&self, request: &ApiRequest, now: u64) -> Result<ApiResponse, ApiError> {
        if request.method != Method::Post {
            return Err(ApiError::Policy(tr(
                "Nur POST-Anfragen sind hier erlaubt.",
                "Only POST requests are allowed here.",
            )));
        }
        self.send_unthrottled(request, now)
    }

    fn send_unthrottled(&self, request: &ApiRequest, now: u64) -> Result<ApiResponse, ApiError> {
        let mut state = self.state.lock().map_err(|_| ApiError::QueueUnavailable)?;
        let response = send_once(&mut state, request, now)?;
        match throttled(&state, &request.account_id, response.status, now) {
            Some(error) => Err(error),
            None => Ok(response),
        }
    }

    /// Each page has its own account-scoped ETag and cached body, including its next link.
    /// A listing longer than the page budget fails with `Truncated`: callers of a complete
    /// listing (such as active runs) must never mistake its first pages for all of it.
    pub fn get_pages(
        &self,
        account_id: &str,
        first: ApiUrl,
        now: u64,
    ) -> Result<Vec<Vec<u8>>, ApiError> {
        complete(self.traverse(account_id, first, now, true, MAX_PAGES)?)
    }

    /// A listing within a page budget (at most the transport's); `truncated` reports a
    /// cut-off listing whose first pages the caller may use as such.
    pub fn get_pages_limited(
        &self,
        account_id: &str,
        first: ApiUrl,
        now: u64,
        max_pages: usize,
    ) -> Result<Pages, ApiError> {
        self.traverse(account_id, first, now, true, max_pages.clamp(1, MAX_PAGES))
    }

    /// Moving discovery windows have unique URLs; retaining their pages would grow without bound.
    /// Budgeted like `get_pages_limited`: a window only needs its newest entries, so the
    /// caller may use the first pages of a cut-off listing.
    #[cfg(feature = "network")]
    pub(crate) fn get_pages_uncached(
        &self,
        account_id: &str,
        first: ApiUrl,
        now: u64,
    ) -> Result<Pages, ApiError> {
        self.traverse(account_id, first, now, false, MAX_PAGES)
    }

    fn traverse(
        &self,
        account_id: &str,
        first: ApiUrl,
        now: u64,
        cache: bool,
        max_pages: usize,
    ) -> Result<Pages, ApiError> {
        let mut state = self.state.lock().map_err(|_| ApiError::QueueUnavailable)?;
        let first_path = first.path().to_owned();
        let mut alias = None;
        let mut next = Some(first);
        let mut seen = BTreeSet::new();
        let mut pages = Pages::default();
        while let Some(url) = next {
            if pages.bodies.len() >= max_pages {
                pages.truncated = true;
                break;
            }
            if !seen.insert(url.clone()) || !same_resource(&first_path, url.path(), &mut alias) {
                return Err(ApiError::Policy(tr(
                    "GitHub-Seitenfolge verlässt die angefragte Ressource.",
                    "GitHub paging leaves the requested resource.",
                )));
            }
            let key = (account_id.to_owned(), url.clone());
            let cached = if cache {
                state.pages.get(&key).cloned()
            } else {
                None
            };
            let mut request = ApiRequest::get(account_id, url)?;
            if let Some(etag) = cached.as_ref().and_then(|cached| cached.etag.as_ref()) {
                request.headers.insert("If-None-Match".into(), etag.clone());
            }
            let response = send_once(&mut state, &request, now)?;
            let page = match response.status {
                200 => CachedPage {
                    etag: checked_etag(response.header("etag"))?,
                    next: next_page(response.header("link"))?,
                    body: response.body,
                },
                304 => {
                    let mut page = cached.ok_or(ApiError::CacheMiss)?;
                    if let Some(link) = response.header("link") {
                        page.next = next_page(Some(link))?;
                    }
                    if response.header("etag").is_some() {
                        page.etag = checked_etag(response.header("etag"))?;
                    }
                    page
                }
                status => {
                    return Err(throttled(&state, account_id, status, now)
                        .unwrap_or(ApiError::Http(status)))
                }
            };
            if page
                .next
                .as_ref()
                .is_some_and(|next| !same_resource(&first_path, next.path(), &mut alias))
            {
                return Err(ApiError::Policy(tr(
                    "GitHub-Seitenfolge wechselt die Ressource.",
                    "GitHub paging switches the resource.",
                )));
            }
            next = page.next.clone();
            pages.bodies.push(page.body.clone());
            if cache {
                state.pages.insert(key, page);
            }
        }
        Ok(pages)
    }

    /// The caller validates workflow eligibility and inputs before constructing this request.
    /// Exactly one transport call is permitted. A lost response is never treated as success.
    pub fn dispatch(
        &self,
        request: &ApiRequest,
        owner: &str,
        repo: &str,
        now: u64,
    ) -> DispatchOutcome {
        if request.method != Method::Post {
            return DispatchOutcome::rejected(tr(
                "Ein Start braucht eine POST-Anfrage.",
                "A start needs a POST request.",
            ));
        }
        match self.execute(request, now) {
            Ok(response) => match parse_dispatch_confirmation(&response, owner, repo) {
                Ok(confirmation) => DispatchOutcome {
                    state: DispatchState::Accepted,
                    run_id: Some(confirmation.workflow_run_id),
                    html_url: Some(confirmation.html_url),
                    message: trf!(
                        "GitHub hat den Start angenommen (Run #{}). Das ist keine Build-Erfolgsmeldung.",
                        "GitHub accepted the start (run #{}). This does not mean the build succeeded.",
                        confirmation.workflow_run_id
                    ),
                },
                Err(_) if matches!(response.status, 400 | 401 | 403 | 404 | 422 | 429) => {
                    DispatchOutcome::rejected(trf!(
                        "GitHub hat den Start abgelehnt (HTTP {}). Es wurde nichts gestartet.",
                        "GitHub rejected the start (HTTP {}). Nothing was started.",
                        response.status
                    ))
                }
                Err(_) => DispatchOutcome::uncertain(tr(
                    "GitHub hat keine Run-ID bestätigt. Prüfe die Runs auf GitHub, bevor du erneut startest.",
                    "GitHub confirmed no run ID. Check the runs on GitHub before you start again.",
                )),
            },
            Err(ApiError::Transport(TransportError::OutcomeUnknown)) => {
                DispatchOutcome::uncertain(tr(
                    "Der Start hat GitHub möglicherweise erreicht. Prüfe die Runs auf GitHub, bevor du es erneut versuchst.",
                    "The start may have reached GitHub. Check the runs on GitHub before you try again.",
                ))
            }
            Err(ApiError::InvalidResponse) => DispatchOutcome::uncertain(tr(
                "Die Antwort auf den Start ließ sich nicht prüfen. Es gibt keinen automatischen neuen Versuch.",
                "The answer to the start could not be checked. There is no automatic retry.",
            )),
            Err(error) => DispatchOutcome::rejected(error.to_string()),
        }
    }
}

/// A backoff deadline further ahead than any backoff can last was set before the clock went
/// back; it has ended, so a backward clock jump never pauses an account longer.
fn backing_off(retry_at: u64, now: u64) -> bool {
    retry_at > now && retry_at <= now.saturating_add(MAX_BACKOFF_SECONDS)
}

fn send_once<T: Transport>(
    state: &mut ClientState<T>,
    request: &ApiRequest,
    now: u64,
) -> Result<ApiResponse, ApiError> {
    if let Some(retry_at) = state.backoff_until.get(&request.account_id) {
        if backing_off(*retry_at, now) {
            return Err(ApiError::RateLimited {
                retry_at: *retry_at,
            });
        }
    }
    let response = state.transport.send(request).map_err(ApiError::Transport)?;
    if response.body.len() > MAX_RESPONSE_BYTES {
        return Err(ApiError::InvalidResponse);
    }
    let primary_exhausted = response.header("x-ratelimit-remaining") == Some("0");
    let secondary = response.status == 429
        || (response.status == 403
            && (response.header("retry-after").is_some()
                || String::from_utf8_lossy(&response.body)
                    .to_ascii_lowercase()
                    .contains("secondary rate limit")));
    // An absolute deadline at or before `now` (a local clock ahead of GitHub's) falls back to
    // the escalating delay; a deadline of now+1 would meet the exhausted quota on every refresh.
    let retry_after = response.header("retry-after").and_then(|value| {
        value
            .trim()
            .parse::<u64>()
            .ok()
            .map(|seconds| now.saturating_add(seconds))
            .or_else(|| {
                chrono::DateTime::parse_from_rfc2822(value)
                    .ok()
                    .and_then(|time| u64::try_from(time.timestamp()).ok())
                    .filter(|deadline| *deadline > now)
            })
    });
    let reset = response
        .header("x-ratelimit-reset")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|reset| *reset > now);
    let succeeded = (200..300).contains(&response.status) || response.status == 304;
    if primary_exhausted || secondary || retry_after.is_some() {
        // Only failed requests escalate the fallback; a 2xx that spent the last unit does not.
        let failures = if succeeded {
            state.rate_limit_failures.remove(&request.account_id);
            0
        } else {
            let failures = state
                .rate_limit_failures
                .entry(request.account_id.clone())
                .or_default();
            let previous = *failures;
            *failures = failures.saturating_add(1);
            previous
        };
        let fallback = now.saturating_add(60u64.saturating_mul(1 << failures.min(4)));
        let until = retry_after
            .into_iter()
            .chain(primary_exhausted.then_some(reset).flatten())
            .max()
            .unwrap_or(fallback)
            .max(now.saturating_add(1))
            .min(now.saturating_add(MAX_BACKOFF_SECONDS));
        state
            .backoff_until
            .insert(request.account_id.clone(), until);
    } else if succeeded {
        state.rate_limit_failures.remove(&request.account_id);
        state.backoff_until.remove(&request.account_id);
    }
    Ok(response)
}

/// A 403/429 that armed this account's backoff is throttling, not an access decision.
fn throttled<T>(
    state: &ClientState<T>,
    account_id: &str,
    status: u16,
    now: u64,
) -> Option<ApiError> {
    if !matches!(status, 403 | 429) {
        return None;
    }
    state
        .backoff_until
        .get(account_id)
        .filter(|retry_at| backing_off(**retry_at, now))
        .map(|retry_at| ApiError::RateLimited {
            retry_at: *retry_at,
        })
}

/// Whether `path` pages the same resource as `first`: `first` itself or, for a
/// `/repos/{owner}/{repo}/<rest>` listing, GitHub's numeric alias `/repositories/{id}/<rest>`.
/// The first alias accepted is pinned, so one traversal never switches between ids.
fn same_resource(first: &str, path: &str, alias: &mut Option<String>) -> bool {
    if path == first {
        return true;
    }
    if let Some(alias) = alias.as_deref() {
        return path == alias;
    }
    let rest = first
        .strip_prefix("/repos/")
        .and_then(|tail| {
            let mut parts = tail.splitn(3, '/');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(owner), Some(repo), Some(rest)) if !owner.is_empty() && !repo.is_empty() => {
                    Some(rest)
                }
                _ => None,
            }
        })
        .filter(|rest| !rest.is_empty());
    let accepted = rest.is_some_and(|rest| {
        path.strip_prefix("/repositories/")
            .and_then(|tail| tail.split_once('/'))
            .is_some_and(|(id, tail)| {
                !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) && tail == rest
            })
    });
    if accepted {
        *alias = Some(path.to_owned());
    }
    accepted
}

fn checked_etag(value: Option<&str>) -> Result<Option<String>, ApiError> {
    if value.is_some_and(|value| {
        value.len() > 512 || value.chars().any(char::is_control) || looks_like_credential(value)
    }) {
        return Err(ApiError::InvalidResponse);
    }
    Ok(value.map(str::to_owned))
}

fn next_page(link: Option<&str>) -> Result<Option<ApiUrl>, ApiError> {
    let mut next = None;
    for part in link.unwrap_or("").split(',') {
        let Some((target, parameters)) = part.trim().split_once(';') else {
            continue;
        };
        let is_next = parameters.split(';').any(|parameter| {
            parameter.trim().strip_prefix("rel=").is_some_and(|rel| {
                rel.trim_matches('"')
                    .split_ascii_whitespace()
                    .any(|rel| rel == "next")
            })
        });
        if is_next {
            if next.is_some() {
                return Err(ApiError::InvalidResponse);
            }
            let url = target
                .trim()
                .strip_prefix('<')
                .and_then(|url| url.strip_suffix('>'))
                .ok_or(ApiError::InvalidResponse)?;
            next = Some(ApiUrl::parse(url)?);
        }
    }
    Ok(next)
}
