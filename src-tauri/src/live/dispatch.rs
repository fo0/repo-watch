//! Workflow starts: validated against the last refresh, claimed before the POST and sent
//! exactly once; an uncertain start stays locked until the user releases it.
use super::LiveStore;
use crate::{
    cache::Release,
    github::{ApiRequest, ApiUrl, Transport},
    i18n::tr,
    models::*,
    workflow::validate_inputs,
};

impl<T: Transport> LiveStore<T> {
    /// Explicit user action after checking the runs on GitHub. It never dispatches, and no
    /// code path calls it automatically.
    pub fn release_uncertain_start(&mut self, start: UncertainStart) -> Result<Snapshot, String> {
        let release = self.cache.release_uncertain_start(&start).map_err(|_| {
            tr(
                "Startsperre konnte nicht aufgehoben werden.",
                "Could not unlock the start.",
            )
        })?;
        self.claims = self.cache.uncertain_starts().map_err(|_| {
            tr(
                "Startsperren konnten nicht geladen werden.",
                "Start locks could not be loaded.",
            )
        })?;
        if release == Release::Replaced {
            return Err(tr(
                "Für diesen Workflow auf diesem Branch oder Tag gibt es eine neuere Startsperre. Prüfe die Runs erneut.",
                "This workflow has a newer start lock on this branch or tag. Check the runs again.",
            )
            .into());
        }
        Ok(self.snapshot())
    }
    /// Validates against the last refresh, reads the workflow at the requested ref (a ref
    /// GitHub no longer knows is rejected here, before any claim), then sends exactly once.
    pub fn dispatch(&mut self, request: DispatchRequest) -> DispatchOutcome {
        if let Err(error) = validate_request_id(&request.request_id) {
            return DispatchOutcome::rejected(error);
        }
        let fingerprint = match serde_json::to_string(&request) {
            Ok(v) => v,
            Err(_) => {
                return DispatchOutcome::rejected(tr("Ungültige Anfrage.", "Invalid request."))
            }
        };
        if let Some((prior, outcome)) = self.outcomes.get(&request.request_id) {
            return if prior == &fingerprint {
                outcome.clone()
            } else {
                DispatchOutcome::rejected(tr(
                    "Start-ID bereits verwendet.",
                    "Start ID already used.",
                ))
            };
        }
        let checked = (|| -> Result<(Repo, crate::github::ApiRequest), String> {
            if self.data.stale {
                return Err(tr("Klicke zuerst auf Aktualisieren.", "Click Refresh first.").into());
            }
            let repo = self
                .data
                .repos
                .iter()
                .find(|r| {
                    r.id == request.repo_id && r.account_id == request.account_id && r.selected
                })
                .ok_or(tr(
                    "Repository nicht im Tray.",
                    "Repository not in the tray.",
                ))?
                .clone();
            if repo.permission != Permission::Write {
                return Err(tr(
                    "Repository hat keinen Schreibzugriff.",
                    "Repository has no write access.",
                )
                .into());
            }
            let workflow = self
                .data
                .workflows
                .iter()
                .find(|w| w.id == request.workflow_id && w.repo_id == repo.id && w.dispatch)
                .ok_or(tr(
                    "Workflow ist nicht manuell startbar.",
                    "Workflow cannot be started manually.",
                ))?;
            let default = self.yaml(&repo, &workflow.path, &repo.default_branch)?;
            if !default.dispatch {
                return Err(tr(
                    "workflow_dispatch fehlt im Standardbranch.",
                    "workflow_dispatch is missing on the default branch.",
                )
                .into());
            }
            if !workflow.refs.contains(&request.git_ref) {
                return Err(tr(
                    "Branch oder Tag unbekannt. Klicke auf Aktualisieren.",
                    "Unknown branch or tag. Click Refresh.",
                )
                .into());
            }
            let mut actual = self.yaml(&repo, &workflow.path, &request.git_ref)?;
            if !actual.dispatch {
                return Err(tr(
                    "workflow_dispatch fehlt im gewählten Branch oder Tag.",
                    "workflow_dispatch is missing on the chosen branch or tag.",
                )
                .into());
            }
            // Environments belong to the repository, not the ref: one the last refresh could
            // not list completely stays unavailable for every ref's environment inputs.
            let unavailable = workflow
                .inputs
                .iter()
                .find(|input| input.r#type == InputType::Environment && input.note.is_some())
                .and_then(|input| input.note.clone());
            for input in &mut actual.inputs {
                if input.r#type == InputType::Environment && unavailable.is_some() {
                    input.options.clear();
                    input.note = unavailable.clone();
                }
            }
            let inputs =
                validate_inputs(&actual.inputs, &request.inputs).map_err(|e| e.to_string())?;
            let api = ApiRequest::workflow_dispatch(
                &repo.account_id,
                &repo.owner,
                &repo.name,
                workflow.id,
                &request.git_ref,
                &inputs,
            )
            .map_err(|e| e.to_string())?;
            Ok((repo, api))
        })();
        let outcome = match checked {
            Err(error) => DispatchOutcome::rejected(error),
            Ok((repo, api)) => self.send_claimed(&request, repo, api),
        };
        let repo = self.data.repos.iter().find(|r| r.id == request.repo_id);
        crate::app_log::info(&format!(
            "dispatch: state={} repo={}",
            match outcome.state {
                DispatchState::Accepted => "accepted",
                DispatchState::Uncertain => "uncertain",
                DispatchState::Rejected => "rejected",
            },
            repo.map_or_else(|| "-".into(), |r| format!("{}/{}", r.owner, r.name))
        ));
        self.outcomes
            .insert(request.request_id, (fingerprint, outcome.clone()));
        outcome
    }
    /// Persists the target claim, sends exactly once, and keeps the target locked whenever
    /// the outcome is unknown or the claim could not be removed.
    fn send_claimed(
        &mut self,
        request: &DispatchRequest,
        repo: Repo,
        api: ApiRequest,
    ) -> DispatchOutcome {
        let target = DispatchTarget {
            account_id: repo.account_id.clone(),
            repo_id: repo.id.clone(),
            workflow_id: request.workflow_id,
            git_ref: request.git_ref.clone(),
        };
        match self
            .cache
            .claim_dispatch(&target, &chrono::Utc::now().to_rfc3339())
        {
            Err(_) => {
                return DispatchOutcome::rejected(tr(
                    "Startsperre konnte nicht gespeichert werden. Es wurde nichts gestartet.",
                    "Start lock could not be saved. Nothing was started.",
                ))
            }
            Ok(None) => {
                return DispatchOutcome::uncertain(tr(
                    "Der letzte Start dieses Workflows auf diesem Branch oder Tag ist ungeklärt. Kein erneuter Start – prüfe die Runs auf GitHub und hebe die Sperre danach auf.",
                    "The last start of this workflow on this branch or tag is uncertain. No new start – check the runs on GitHub, then unlock it.",
                ))
            }
            Ok(Some(claim)) => self.claims.push(claim),
        }
        let mut outcome = self
            .client
            .dispatch(&api, &repo.owner, &repo.name, (self.clock)());
        if outcome.state != DispatchState::Uncertain {
            match self.cache.resolve_dispatch(&target) {
                Ok(()) => self.claims.retain(|claim| claim.target() != target),
                Err(_) => outcome.message.push_str(tr(
                    " Die Startsperre konnte nicht aufgehoben werden; prüfe die Runs auf GitHub und hebe sie danach auf.",
                    " Could not unlock the start; check the runs on GitHub, then unlock it.",
                )),
            }
        }
        if outcome.state == DispatchState::Accepted {
            if let (Some(id), Some(html_url)) = (outcome.run_id, outcome.html_url.as_ref()) {
                if let Ok(run_url) = ApiUrl::parse(&format!(
                    "https://api.github.com/repos/{}/{}/actions/runs/{id}",
                    repo.owner, repo.name
                )) {
                    self.tracked.insert(
                        id,
                        crate::github::RunTracker {
                            account_id: repo.account_id.clone(),
                            repo_id: repo.id.clone(),
                            workflow_id: request.workflow_id,
                            confirmation: crate::github::DispatchConfirmation {
                                workflow_run_id: id,
                                run_url,
                                html_url: html_url.clone(),
                            },
                        },
                    );
                }
            }
            self.poll_at.clear();
            self.refresh();
        }
        outcome
    }
}
