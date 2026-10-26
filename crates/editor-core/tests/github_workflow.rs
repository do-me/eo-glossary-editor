use editor_core::{
    github::{Github, bundled, record_operation},
    model::*,
    source::Edit,
    store::Store,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{method, path},
};
fn operation() -> Operation {
    Operation {
        repository: UPSTREAM.into(),
        base_branch: "main".into(),
        id: "operation-1".into(),
        title: "Clarify terminology".into(),
        body: "Scientific corrections.".into(),
        original_files: BTreeMap::new(),
        files: BTreeMap::from([
            ("docs/terms/accuracy.md".into(), "updated accuracy".into()),
            ("docs/terms/precision.md".into(), "updated precision".into()),
        ]),
        head_repo: "expert/eo-glossary".into(),
        head_branch: "eo-editor/session-1".into(),
        parent: "parent-sha".into(),
        commit: Some("commit-sha".into()),
        account_id: 123,
    }
}
fn pr() -> Value {
    json!({"number":99,"title":"Clarify terminology","state":"open","user":{"id":123},"head":{"sha":"commit-sha","ref":"eo-editor/session-1","repo":{"full_name":"expert/eo-glossary"}},"base":{"repo":{"full_name":UPSTREAM}},"merged_at":null})
}
#[tokio::test]
async fn response_loss_after_pr_creation_recovers_same_pr_without_duplicate() {
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    let op = operation();
    let created = Arc::new(AtomicBool::new(false));
    Mock::given(method("GET"))
        .and(path(
            "/repos/expert/eo-glossary/git/ref/heads/eo-editor/session-1",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"object":{"sha":"commit-sha"}})),
        )
        .mount(&server)
        .await;
    let flag = created.clone();
    Mock::given(method("GET"))
        .and(path("/repos/ceos-org/eo-glossary/pulls"))
        .respond_with(move |_: &Request| {
            ResponseTemplate::new(200).set_body_json(if flag.load(Ordering::SeqCst) {
                json!([pr()])
            } else {
                json!([])
            })
        })
        .mount(&server)
        .await;
    let flag = created.clone();
    Mock::given(method("POST"))
        .and(path("/repos/ceos-org/eo-glossary/pulls"))
        .respond_with(move |_: &Request| {
            flag.store(true, Ordering::SeqCst);
            ResponseTemplate::new(502)
                .set_body_json(json!({"message":"Response lost after creating PR"}))
        })
        .expect(1)
        .mount(&server)
        .await;
    assert!(github.publish(&op, None, "test-token").await.is_err());
    let recovered = github.publish(&op, None, "test-token").await.unwrap();
    assert_eq!(recovered.number, 99);
    assert_eq!(
        recovered.url,
        "https://github.com/ceos-org/eo-glossary/pull/99"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(!requests.iter().any(|r| r.method == "PATCH"));
    assert!(
        requests
            .iter()
            .all(|r| r.headers.get("authorization").unwrap() == "Bearer test-token")
    );
}
#[tokio::test]
async fn creates_one_tree_and_commit_for_multiple_terms() {
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    let op = operation();
    Mock::given(method("GET"))
        .and(path("/repos/expert/eo-glossary/git/commits/parent-sha"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"tree":{"sha":"base-tree"}})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/repos/expert/eo-glossary/git/trees"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"sha":"new-tree"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/repos/expert/eo-glossary/git/commits"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"sha":"new-commit"})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        github.create_commit(&op, "test-token").await.unwrap(),
        "new-commit"
    );
    let requests = server.received_requests().await.unwrap();
    let tree: Value = requests
        .iter()
        .find(|r| r.url.path().ends_with("/trees"))
        .unwrap()
        .body_json()
        .unwrap();
    assert_eq!(tree["tree"].as_array().unwrap().len(), 2);
    assert_eq!(tree["base_tree"], "base-tree");
}
#[tokio::test]
async fn remote_head_change_never_force_pushes_or_creates_pr() {
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    Mock::given(method("GET"))
        .and(path(
            "/repos/expert/eo-glossary/git/ref/heads/eo-editor/session-1",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"object":{"sha":"someone-elses-commit"}})),
        )
        .mount(&server)
        .await;
    assert_eq!(
        github
            .publish(&operation(), None, "token")
            .await
            .unwrap_err()
            .code,
        "remote_changed"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
#[test]
fn interrupted_operation_survives_restart_and_only_frozen_edits_are_marked_submitted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let catalogue = bundled().unwrap();
    let mut store = Store::open(&path, &catalogue).unwrap();
    let session = store.create_session("Review".into()).unwrap();
    let term = "docs/terms/accuracy.md";
    let saved = store
        .save_edit(
            &session.id,
            term,
            0,
            Edit::Section {
                id: 1,
                markdown: "First frozen definition.".into(),
            },
        )
        .unwrap();
    let mut op = operation();
    op.files = BTreeMap::from([(term.into(), saved.document.source.clone())]);
    op.original_files = op.files.clone();
    record_operation(&mut store, &session.id, &op).unwrap();
    let newer = store
        .save_edit(
            &session.id,
            term,
            1,
            Edit::Section {
                id: 1,
                markdown: "Newer local definition.".into(),
            },
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(&path, &catalogue).unwrap();
    assert_eq!(
        store.session(&session.id).unwrap().operation.unwrap().id,
        op.id
    );
    let link = Submission {
        number: 99,
        url: "https://github.com/ceos-org/eo-glossary/pull/99".into(),
        head_repo: op.head_repo.clone(),
        head_branch: op.head_branch.clone(),
        head_sha: "commit-sha".into(),
        account_id: 123,
        state: "open".into(),
    };
    store.finish_submission(&session.id, &op, link).unwrap();
    let final_session = store.session(&session.id).unwrap();
    assert!(final_session.operation.is_none());
    assert_eq!(
        final_session.terms[term].submitted,
        Some(saved.document.source)
    );
    assert_eq!(final_session.terms[term].source, newer.document.source);
    assert!(final_session.terms[term].pending());
}
#[tokio::test]
async fn ordinary_contributor_gets_personal_fork_and_one_frozen_operation_for_many_files() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    let catalogue = bundled().unwrap();
    let mut session = Session::new("WGCV contribution".into(), &catalogue);
    let account = Account {
        id: 123,
        login: "expert".into(),
        name: None,
    };
    Mock::given(method("GET"))
        .and(path("/repos/ceos-org/eo-glossary"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"permissions":{"push":false}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/expert/eo-glossary"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message":"Not Found"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/repos/ceos-org/eo-glossary/forks"))
        .respond_with(
            ResponseTemplate::new(202).set_body_json(json!({"full_name":"expert/eo-glossary"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/ceos-org/eo-glossary/commits/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"sha":catalogue.sha})))
        .mount(&server)
        .await;
    for term in ["docs/terms/accuracy.md", "docs/terms/precision.md"] {
        let starting = catalogue.files[term].clone();
        let source = editor_core::source::apply_edit(
            &starting,
            Edit::Section {
                id: 1,
                markdown: "A focused scientific clarification.".into(),
            },
        )
        .unwrap();
        Mock::given(method("GET"))
            .and(path(format!("/repos/expert/eo-glossary/contents/{term}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"content":STANDARD.encode(&starting)})),
            )
            .mount(&server)
            .await;
        session.terms.insert(
            term.into(),
            Draft {
                starting,
                source,
                submitted: None,
                revision: 1,
                is_new: false,
            },
        );
    }
    let prepared = github
        .prepare(
            &session,
            &account,
            "token",
            "Clarify two terms".into(),
            "Sources reviewed.".into(),
        )
        .await
        .unwrap();
    assert_eq!(prepared.head_repo, "expert/eo-glossary");
    assert_eq!(prepared.head_branch, format!("eo-editor/{}", session.id));
    assert_eq!(prepared.files.len(), 2);
    assert_eq!(prepared.files, prepared.original_files);
}
#[tokio::test]
async fn later_submission_updates_existing_pr_with_non_forced_reference_update() {
    use wiremock::matchers::body_json;
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    let op = operation();
    let link = Submission {
        number: 99,
        url: "https://github.com/ceos-org/eo-glossary/pull/99".into(),
        head_repo: op.head_repo.clone(),
        head_branch: op.head_branch.clone(),
        head_sha: "parent-sha".into(),
        account_id: 123,
        state: "open".into(),
    };
    Mock::given(method("GET"))
        .and(path("/repos/ceos-org/eo-glossary/pulls/99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/repos/expert/eo-glossary/git/ref/heads/eo-editor/session-1",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"object":{"sha":"parent-sha"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(
            "/repos/expert/eo-glossary/git/refs/heads/eo-editor/session-1",
        ))
        .and(body_json(json!({"sha":"commit-sha","force":false})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        github
            .publish(&op, Some(&link), "token")
            .await
            .unwrap()
            .number,
        99
    );
    assert!(
        !server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|r| r.method == "POST")
    );
}
#[tokio::test]
async fn configured_fork_receives_pr_and_updates_without_upstream_requests() {
    use wiremock::matchers::body_json;
    let server = MockServer::start().await;
    let github = Github::new(&server.uri());
    let mut op = operation();
    op.repository = "do-me/eo-glossary".into();
    op.head_repo = op.repository.clone();
    op.base_branch = "review-base".into();
    let mut response = pr();
    response["base"]["repo"]["full_name"] = json!(op.repository);
    response["head"]["repo"]["full_name"] = json!(op.repository);
    Mock::given(method("GET"))
        .and(path(
            "/repos/do-me/eo-glossary/git/ref/heads/eo-editor/session-1",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"object":{"sha":"commit-sha"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/do-me/eo-glossary/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/repos/do-me/eo-glossary/pulls")).and(body_json(json!({"title":op.title,"body":op.body,"head":"do-me:eo-editor/session-1","base":"review-base","maintainer_can_modify":true}))).respond_with(ResponseTemplate::new(201).set_body_json(response.clone())).expect(1).mount(&server).await;
    let link = github.publish(&op, None, "test-token").await.unwrap();
    assert_eq!(link.url, "https://github.com/do-me/eo-glossary/pull/99");
    Mock::given(method("GET"))
        .and(path("/repos/do-me/eo-glossary/pulls/99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    assert_eq!(
        github
            .publish(&op, Some(&link), "test-token")
            .await
            .unwrap()
            .url,
        link.url
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.url.path().contains("ceos-org"))
    );
}
