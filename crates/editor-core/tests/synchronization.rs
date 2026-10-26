use editor_core::{
    github::bundled,
    model::*,
    source::{self, Edit},
    store::Store,
    sync::Choice,
};
const TERM: &str = "docs/terms/accuracy.md";
fn edit(id: usize, text: &str) -> Edit {
    Edit::Section {
        id,
        markdown: text.into(),
    }
}
fn advance(store: &mut Store, base: &Catalogue, id: usize, text: &str, digit: char) -> Catalogue {
    let mut latest = base.clone();
    latest.sha = digit.to_string().repeat(40);
    latest.fetched_at = now();
    latest.files.insert(
        TERM.into(),
        source::apply_edit(&base.files[TERM], edit(id, text)).unwrap(),
    );
    store.cache_remote(&latest).unwrap();
    latest
}
#[test]
fn main_updates_merge_non_overlapping_edits_and_update_the_searchable_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let base = bundled().unwrap();
    let mut store = Store::open(&dir.path().join("test.db"), &base).unwrap();
    let s = store.create_session("My review".into()).unwrap();
    store
        .save_edit(&s.id, TERM, 0, edit(1, "My precise definition."))
        .unwrap();
    let latest = advance(&mut store, &base, 2, "A note approved in another PR.", 'a');
    let result = store.apply_updates(&s.id, &latest.sha).unwrap();
    assert!(result.conflicts.is_empty());
    assert_eq!(result.merged, 1);
    let restored = store.open_term(&s.id, TERM).unwrap();
    assert!(restored.document.source.contains("My precise definition."));
    assert!(
        restored
            .document
            .source
            .contains("A note approved in another PR.")
    );
    let session = store.session(&s.id).unwrap();
    assert_eq!(session.base_sha, latest.sha);
    assert_eq!(session.terms[TERM].starting, latest.files[TERM]);
    assert!(store.export(&s.id).is_ok());
    let revision = restored.revision;
    store.apply_updates(&s.id, &latest.sha).unwrap();
    assert_eq!(store.open_term(&s.id, TERM).unwrap().revision, revision);
}
#[tokio::test]
async fn overlapping_edits_remain_local_until_a_version_is_explicitly_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let base = bundled().unwrap();
    let mut store = Store::open(&path, &base).unwrap();
    let s = store.create_session("Conflicting review".into()).unwrap();
    let mine = store
        .save_edit(&s.id, TERM, 0, edit(1, "My local definition."))
        .unwrap();
    let latest = advance(
        &mut store,
        &base,
        1,
        "Definition merged from another PR.",
        'b',
    );
    let report = store.apply_updates(&s.id, &latest.sha).unwrap();
    assert_eq!(report.conflicts.len(), 1);
    let conflict = &report.conflicts[0];
    assert!(conflict.local_diff.contains("My local definition."));
    assert!(
        conflict
            .github_diff
            .contains("Definition merged from another PR.")
    );
    assert_eq!(
        store.open_term(&s.id, TERM).unwrap().document.source,
        mine.document.source
    );
    let account = Account {
        id: 1,
        login: "expert".into(),
        name: None,
    };
    let error = editor_core::github::Github::default()
        .prepare(
            &store.session(&s.id).unwrap(),
            &account,
            "never-sent",
            "Review".into(),
            "".into(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "sync_conflicts");
    let report = store
        .resolve_update(
            &s.id,
            TERM,
            conflict.revision,
            &conflict.github_sha,
            Choice::Local,
        )
        .unwrap();
    assert!(report.conflicts.is_empty());
    drop(store);
    let store = Store::open(&path, &base).unwrap();
    assert!(
        store
            .open_term(&s.id, TERM)
            .unwrap()
            .document
            .source
            .contains("My local definition.")
    );
    assert_eq!(
        store.session(&s.id).unwrap().terms[TERM].starting,
        latest.files[TERM]
    );
}
#[test]
fn pulling_github_is_revision_checked_and_preserves_the_overwritten_revision() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let base = bundled().unwrap();
    let mut store = Store::open(&db, &base).unwrap();
    let s = store.create_session("Review".into()).unwrap();
    store.save_edit(&s.id, TERM, 0, edit(1, "Mine")).unwrap();
    let latest = advance(&mut store, &base, 1, "GitHub version", 'c');
    let report = store.apply_updates(&s.id, &latest.sha).unwrap();
    let c = &report.conflicts[0];
    let edited = store
        .save_edit(&s.id, TERM, c.revision, edit(1, "My newer local draft"))
        .unwrap();
    assert_eq!(
        store
            .resolve_update(&s.id, TERM, c.revision, &c.github_sha, Choice::Github)
            .unwrap_err()
            .code,
        "stale_revision"
    );
    store
        .resolve_update(&s.id, TERM, edited.revision, &c.github_sha, Choice::Github)
        .unwrap();
    assert_eq!(
        store.open_term(&s.id, TERM).unwrap().document.source,
        latest.files[TERM]
    );
    assert!(store.changes(&s.id, false).unwrap().is_empty());
    drop(store);
    let connection = rusqlite::Connection::open(db).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM revisions WHERE source LIKE '%My newer local draft%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count > 0);
}
#[test]
fn a_new_remote_version_invalidates_an_old_conflict_choice() {
    let dir = tempfile::tempdir().unwrap();
    let base = bundled().unwrap();
    let mut store = Store::open(&dir.path().join("test.db"), &base).unwrap();
    let s = store.create_session("Review".into()).unwrap();
    store.save_edit(&s.id, TERM, 0, edit(1, "Mine")).unwrap();
    let first = advance(&mut store, &base, 1, "First remote", 'd');
    let old = store.apply_updates(&s.id, &first.sha).unwrap();
    let second = advance(&mut store, &first, 1, "Second remote", 'e');
    let new = store.apply_updates(&s.id, &second.sha).unwrap();
    assert_eq!(new.conflicts.len(), 1);
    assert!(new.conflicts[0].github_diff.contains("Second remote"));
    assert!(
        store
            .resolve_update(
                &s.id,
                TERM,
                old.conflicts[0].revision,
                &old.conflicts[0].github_sha,
                Choice::Local
            )
            .is_err()
    );
}
#[test]
fn collisions_with_new_terms_and_remote_deletions_have_explicit_choices() {
    let dir = tempfile::tempdir().unwrap();
    let base = bundled().unwrap();
    let mut store = Store::open(&dir.path().join("test.db"), &base).unwrap();
    let s = store.create_session("Review".into()).unwrap();
    let new = store.new_term(&s.id, "A new term", &[]).unwrap();
    let mut latest = base.clone();
    latest.sha = "f".repeat(40);
    latest.files.insert(
        new.path.clone(),
        source::apply_edit(&new.document.source, edit(1, "Published meanwhile")).unwrap(),
    );
    latest.files.remove(TERM);
    store
        .save_edit(&s.id, TERM, 0, edit(1, "My old term edit"))
        .unwrap();
    store.cache_remote(&latest).unwrap();
    let report = store.apply_updates(&s.id, &latest.sha).unwrap();
    assert_eq!(report.conflicts.len(), 2);
    for c in report.conflicts {
        store
            .resolve_update(&s.id, &c.path, c.revision, &c.github_sha, Choice::Github)
            .unwrap();
    }
    assert!(store.open_term(&s.id, TERM).is_err());
    assert!(
        store
            .open_term(&s.id, &new.path)
            .unwrap()
            .document
            .source
            .contains("Published meanwhile")
    );
    assert!(store.session(&s.id).unwrap().conflicts.is_empty());
}
#[test]
fn zoom_and_workspace_restore_without_affecting_other_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let base = bundled().unwrap();
    let mut store = Store::open(&db, &base).unwrap();
    let s = store.create_session("First".into()).unwrap();
    let other = store.create_session("Second".into()).unwrap();
    let mut workspace = Workspace {
        view: "review".into(),
        filters_open: true,
        review_mode: "compare".into(),
        contribution_title: "A coherent contribution".into(),
        contribution_body: "Sources reviewed.".into(),
        ..Default::default()
    };
    workspace.tabs = vec![TERM.into()];
    workspace.active = Some(TERM.into());
    workspace.scroll.insert(TERM.into(), 348.0);
    workspace.view_scroll.insert("review".into(), 201.0);
    workspace.term_ui.insert(
        TERM.into(),
        TermUi {
            details: true,
            source: true,
            source_sections: vec![1],
        },
    );
    store.save_workspace(&s.id, workspace).unwrap();
    store.save_preferences(Preferences { zoom: 140 }).unwrap();
    assert!(store.save_preferences(Preferences { zoom: 999 }).is_err());
    drop(store);
    let store = Store::open(&db, &base).unwrap();
    assert_eq!(store.preferences().unwrap().zoom, 140);
    let workspace = store.session(&s.id).unwrap().workspace;
    assert_eq!(workspace.view, "review");
    assert_eq!(workspace.scroll[TERM], 348.0);
    assert_eq!(workspace.contribution_title, "A coherent contribution");
    assert!(workspace.term_ui[TERM].details);
    assert_eq!(workspace.term_ui[TERM].source_sections, vec![1]);
    assert!(store.session(&other.id).unwrap().workspace.tabs.is_empty());
}

#[test]
fn removing_a_submitted_term_preserves_a_renderable_draft_and_requires_a_choice() {
    let base = bundled().unwrap();
    let mut session = Session::new("Submitted review".into(), &base);
    session.terms.insert(
        TERM.into(),
        Draft {
            starting: base.files[TERM].clone(),
            source: base.files[TERM].clone(),
            submitted: Some(base.files[TERM].clone()),
            revision: 2,
            is_new: false,
        },
    );
    let mut latest = base.clone();
    latest.sha = "9".repeat(40);
    latest.files.remove(TERM);
    editor_core::sync::reconcile(&mut session, &base, &latest).unwrap();
    assert_eq!(session.terms[TERM].source, base.files[TERM]);
    let report = editor_core::sync::report(&session, now(), 0, 0).unwrap();
    assert_eq!(report.conflicts.len(), 1);
    assert!(report.conflicts[0].deleted);
    assert_eq!(report.conflicts[0].title, "Accuracy");
}

#[test]
fn device_authorization_uses_registration_permissions_for_apps_and_scopes_for_oauth() {
    assert_eq!(
        editor_core::github::device_request("public-id", true),
        serde_json::json!({"client_id":"public-id"})
    );
    assert_eq!(
        editor_core::github::device_request("public-id", false),
        serde_json::json!({"client_id":"public-id","scope":"public_repo"})
    );
}
