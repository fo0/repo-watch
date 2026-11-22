//! Pagination through GitHub's numeric repository alias `repositories/{id}/<rest>`.
use super::super::*;
use super::{client, response};

const FIRST: &str = "https://api.github.com/repos/demo/project/actions/runs?per_page=100";

fn next(path_and_query: &str) -> String {
    format!("<https://api.github.com/{path_and_query}>; rel=\"next\"")
}

#[test]
fn pagination_follows_the_numeric_repository_alias_and_revalidates_its_cached_pages() {
    let second = next("repositories/123/actions/runs?per_page=100&page=2");
    let third = next("repositories/123/actions/runs?per_page=100&page=3");
    let (client, sent) = client(vec![
        Ok(response(
            200,
            &[("ETag", "\"p1\""), ("Link", second.as_str())],
            "[1]",
        )),
        Ok(response(
            200,
            &[("ETag", "\"p2\""), ("Link", third.as_str())],
            "[2]",
        )),
        Ok(response(200, &[("ETag", "\"p3\"")], "[3]")),
        Ok(response(304, &[], "")),
        Ok(response(304, &[], "")),
        Ok(response(304, &[], "")),
    ]);
    let expected = vec![b"[1]".to_vec(), b"[2]".to_vec(), b"[3]".to_vec()];
    for now in [0, 1] {
        assert_eq!(
            client
                .get_pages("fixture", ApiUrl::parse(FIRST).unwrap(), now)
                .unwrap(),
            expected
        );
    }
    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 6);
    assert_eq!(
        sent[1].url().as_str(),
        "https://api.github.com/repositories/123/actions/runs?per_page=100&page=2"
    );
    // The second traversal reaches the alias pages through their cached next links.
    assert_eq!(sent[3].headers()["If-None-Match"], "\"p1\"");
    assert_eq!(sent[4].headers()["If-None-Match"], "\"p2\"");
    assert_eq!(sent[5].headers()["If-None-Match"], "\"p3\"");
    assert_eq!(
        sent[5].url().as_str(),
        "https://api.github.com/repositories/123/actions/runs?per_page=100&page=3"
    );
}

#[test]
fn pagination_rejects_aliases_of_other_resources_ids_or_repositories() {
    let cases: [(&str, &[&str]); 5] = [
        // Same id, different rest.
        (FIRST, &["repositories/123/actions/workflows?page=2"]),
        // A second id mid-traversal.
        (
            FIRST,
            &[
                "repositories/123/actions/runs?per_page=100&page=2",
                "repositories/456/actions/runs?per_page=100&page=3",
            ],
        ),
        // A non-numeric id.
        (FIRST, &["repositories/demo/actions/runs?page=2"]),
        // Another repository by name.
        (FIRST, &["repos/other/repo/actions/runs?page=2"]),
        // Only `repos/{owner}/{repo}/<rest>` listings have an alias.
        (
            "https://api.github.com/user/repos?page=1",
            &["repositories/123/repos?page=2"],
        ),
    ];
    for (first, links) in cases {
        let responses = links
            .iter()
            .map(|link| Ok(response(200, &[("Link", next(link).as_str())], "[]")))
            .collect();
        let (client, sent) = client(responses);
        let error = client
            .get_pages("fixture", ApiUrl::parse(first).unwrap(), 0)
            .unwrap_err();
        assert!(matches!(error, ApiError::Policy(_)), "{links:?}");
        assert_eq!(sent.lock().unwrap().len(), links.len(), "{links:?}");
    }
}
