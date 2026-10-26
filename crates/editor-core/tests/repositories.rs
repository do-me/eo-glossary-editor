use editor_core::{
    github::bundled,
    model::*,
    repository,
    source::Edit,
    store::{SessionExport, Store},
};
use serde_json::json;
const FORK: &str = "do-me/eo-glossary";
#[test]
fn canonical_repository_urls_do_not_become_arbitrary_api_hosts() {
    for input in [
        " https://github.com/do-me/eo-glossary/ ",
        "https://github.com/DO-ME/EO-GLOSSARY.git",
        "do-me/eo-glossary",
    ] {
        assert_eq!(repository::parse(input).unwrap(), FORK);
    }
    for input in [
        "https://evil.test/do-me/eo-glossary",
        "https://github.com.evil.test/do-me/eo-glossary",
        "https://secret@github.com/do-me/eo-glossary",
        "https://github.com/do-me/eo-glossary/tree/main",
        "https://github.com/do-me/eo-glossary?token=secret",
        "file:///tmp/glossary",
        "ceos-org/../private",
        "https://github.com:444/do-me/eo-glossary",
        "https://github.com/do-me/%2e%2e",
    ] {
        assert!(repository::parse(input).is_err(), "{input}");
    }
}
#[test]
fn switching_same_sha_repositories_keeps_drafts_baselines_and_followups_separate() {
    let dir = tempfile::tempdir().unwrap();
    let upstream = bundled().unwrap();
    let mut fork = upstream.clone();
    fork.repository = FORK.into();
    fork.files.insert(
        "docs/terms/accuracy.md".into(),
        editor_core::source::skeleton("Fork accuracy", &[]).unwrap(),
    );
    let mut store = Store::open(&dir.path().join("sessions.db"), &upstream).unwrap();
    let original = store.create_session("Original work".into()).unwrap();
    let edited = store
        .save_edit(
            &original.id,
            "docs/terms/accuracy.md",
            0,
            Edit::Section {
                id: 1,
                markdown: "Retained original-repository draft.".into(),
            },
        )
        .unwrap();
    let test = store.switch_repository(&fork).unwrap();
    assert_eq!(test.repository, FORK);
    assert_eq!(store.catalogue(None).unwrap().repository, FORK);
    assert_eq!(
        store
            .open_term(&test.id, "docs/terms/accuracy.md")
            .unwrap()
            .document
            .title,
        "Fork accuracy"
    );
    assert_eq!(
        store
            .open_term(&original.id, "docs/terms/accuracy.md")
            .unwrap()
            .document
            .source,
        edited.document.source
    );
    let original_export: SessionExport =
        serde_json::from_str(&store.export(&original.id).unwrap()).unwrap();
    assert_eq!(original_export.catalogue.repository, UPSTREAM);
    assert_eq!(
        original_export.catalogue.files["docs/terms/accuracy.md"],
        upstream.files["docs/terms/accuracy.md"]
    );
    assert_eq!(store.follow_up(&original.id).unwrap().repository, UPSTREAM);
    assert_eq!(
        store.create_session("Test".into()).unwrap().repository,
        FORK
    );
    // New term lookup also uses the active session's own snapshot, even if settings differ.
    let new = store
        .new_term(&original.id, "A distinct source term", &[])
        .unwrap();
    assert!(new.is_new);
    let fork_export = store.export(&test.id).unwrap();
    let mut other = Store::open(&dir.path().join("import.db"), &upstream).unwrap();
    let imported = other.import(&fork_export).unwrap();
    assert_eq!(imported.repository, FORK);
    assert_eq!(
        other
            .open_term(&imported.id, "docs/terms/accuracy.md")
            .unwrap()
            .document
            .title,
        "Fork accuracy"
    );
}
#[test]
fn migration_preserves_version_one_sessions_and_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let catalogue = bundled().unwrap();
    let session = Session::new("Existing work".into(), &catalogue);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE snapshots(sha TEXT PRIMARY KEY,data TEXT NOT NULL);CREATE TABLE sessions(id TEXT PRIMARY KEY,data TEXT NOT NULL,updated TEXT NOT NULL);CREATE TABLE preferences(key TEXT PRIMARY KEY,data TEXT NOT NULL);CREATE TABLE revisions(session TEXT,path TEXT,revision INTEGER,source TEXT,created TEXT,PRIMARY KEY(session,path,revision));PRAGMA user_version=1;").unwrap();
    connection
        .execute(
            "INSERT INTO snapshots VALUES(?1,?2)",
            (&catalogue.sha, serde_json::to_string(&catalogue).unwrap()),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO sessions VALUES(?1,?2,?3)",
            (
                &session.id,
                serde_json::to_string(&session).unwrap(),
                &session.updated_at,
            ),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO preferences VALUES('catalogue',?1)",
            [&catalogue.sha],
        )
        .unwrap();
    drop(connection);
    let mut store = Store::open(&path, &catalogue).unwrap();
    assert_eq!(store.session(&session.id).unwrap().name, "Existing work");
    assert_eq!(store.catalogue(None).unwrap().repository, UPSTREAM);
    let mut fork = catalogue.clone();
    fork.repository = FORK.into();
    store.switch_repository(&fork).unwrap();
    assert_eq!(
        store
            .open_term(&session.id, "docs/terms/accuracy.md")
            .unwrap()
            .document
            .title,
        "Accuracy"
    );
    drop(store);
    let reopened = Store::open(&path, &catalogue).unwrap();
    assert_eq!(reopened.catalogue(None).unwrap().repository, FORK);
}
#[test]
fn identical_pr_numbers_in_different_repositories_do_not_deduplicate() {
    let dir = tempfile::tempdir().unwrap();
    let upstream = bundled().unwrap();
    let mut fork = upstream.clone();
    fork.repository = FORK.into();
    let mut store = Store::open(&dir.path().join("test.db"), &upstream).unwrap();
    for catalogue in [upstream, fork] {
        let mut session = Session::new("PR 13".into(), &catalogue);
        session.submission = Some(Submission {
            number: 13,
            url: format!("https://github.com/{}/pull/13", catalogue.repository),
            head_repo: catalogue.repository.clone(),
            head_branch: "review".into(),
            head_sha: catalogue.sha.clone(),
            account_id: 1,
            state: "open".into(),
        });
        store.import_pr(catalogue, session).unwrap();
    }
    assert_eq!(store.sessions().unwrap().len(), 2);
    assert_eq!(store.find_pr(FORK, 13).unwrap().unwrap().repository, FORK);
    assert_eq!(
        store.find_pr(UPSTREAM, 13).unwrap().unwrap().repository,
        UPSTREAM
    );
    let serialized = serde_json::to_value(store.find_pr(FORK, 13).unwrap().unwrap()).unwrap();
    assert_eq!(
        serialized["submission"]["url"],
        json!("https://github.com/do-me/eo-glossary/pull/13")
    );
}
#[test]
fn rendered_review_excludes_yaml_and_preserves_definitions_tables_and_citations() {
    let catalogue = bundled().unwrap();
    for (path, source) in catalogue.files {
        let page = editor_core::review::page(&source).unwrap();
        assert!(!page.sections.is_empty(), "{path}");
        assert!(
            page.sections.iter().all(|s| !s.html.contains("tags:\n")),
            "{path}"
        );
    }
    let text = editor_core::source::skeleton("Rendered term", &[]).unwrap();
    let text=editor_core::source::apply_edit(&text,Edit::Section{id:1,markdown:"A **clear** definition with a [source](https://example.org).\n\n| Parameter | Value |\n| --- | --- |\n| Wavelength | 2 μm |".into()}).unwrap();
    let page = editor_core::review::page(&text).unwrap();
    assert!(page.sections[0].html.contains("<strong>clear</strong>"));
    assert!(page.sections[0].html.contains("<table>"));
    assert!(
        page.sections[0]
            .html
            .contains("href=\"https://example.org\"")
    );
    assert_eq!(page.sections.len(), 4);
}
