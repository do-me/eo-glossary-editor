use editor_core::{
    github::{bundled, merge_source},
    source::{self, Edit},
    store::Store,
};
fn section_edit(id: usize, markdown: &str) -> Edit {
    Edit::Section {
        id,
        markdown: markdown.into(),
    }
}
#[test]
fn every_real_term_preserves_exact_source_and_protected_headings() {
    let catalogue = bundled().unwrap();
    assert_eq!(catalogue.files.len(), 509);
    for (path, source) in catalogue.files {
        source::validate_path(&path).unwrap();
        let doc = source::parse(&source).unwrap();
        let untouched = source::apply_edit(
            &source,
            Edit::Metadata {
                description: doc.description.clone(),
                tags: doc.tags.clone(),
            },
        )
        .unwrap();
        assert_eq!(source, untouched, "{path}");
        for section in &doc.sections {
            if section.editable {
                assert_eq!(
                    source::apply_edit(&source, section_edit(section.id, &section.markdown))
                        .unwrap(),
                    source,
                    "{path}: {}",
                    section.heading
                );
                for fragment in [
                    "# A replacement title",
                    "## 1 Definition",
                    "### Sources",
                    "---",
                    "Definition\n===",
                ] {
                    assert_eq!(
                        source::apply_edit(&source, section_edit(section.id, fragment))
                            .unwrap_err()
                            .code,
                        "protected_heading",
                        "{path}"
                    );
                }
                let edited = source::apply_edit(
                    &source,
                    section_edit(section.id, "A precise scientific definition."),
                )
                .unwrap();
                assert_eq!(
                    &source[..section.content_range.start],
                    &edited[..section.content_range.start]
                );
                assert!(edited.ends_with(&source[section.content_range.end..]));
                let newdoc = source::parse(&edited).unwrap();
                assert_eq!(
                    doc.sections
                        .iter()
                        .map(|s| (&s.heading, s.level))
                        .collect::<Vec<_>>(),
                    newdoc
                        .sections
                        .iter()
                        .map(|s| (&s.heading, s.level))
                        .collect::<Vec<_>>(),
                    "{path}"
                );
            } else {
                assert_eq!(
                    source::apply_edit(&source, section_edit(section.id, "Modified title"))
                        .unwrap_err()
                        .code,
                    "protected_heading"
                );
            }
        }
        let edited = source::apply_edit(
            &source,
            Edit::Metadata {
                description: "Test description".into(),
                tags: doc.tags,
            },
        )
        .unwrap();
        assert_eq!(
            source.split_once("\n#").unwrap().1,
            edited.split_once("\n#").unwrap().1,
            "{path}"
        );
    }
}
#[test]
fn unknown_frontmatter_and_custom_tags_survive_metadata_changes() {
    let text = "---\ntitle: 'Scientific term'\ndescription: \"A multiline\n  description with a colon: yes\"\ncustom:\n  id: 42\ntags:\n- core\n- custom-team\n- to be discussed\n---\n\n# Scientific term\n\n## 1 Definition\n\nKept **exactly**.\n\n### Notes\n\n### Examples\n\n### Sources\n";
    let doc = source::parse(text).unwrap();
    let result = source::apply_edit(
        text,
        Edit::Metadata {
            description: "Changed".into(),
            tags: doc.tags.clone(),
        },
    )
    .unwrap();
    assert!(result.contains("custom:\n  id: 42\n"));
    assert!(result.contains("- custom-team\n"));
    assert!(result.ends_with(text.split_once("---\n\n").unwrap().1));
}
#[test]
fn skeleton_ready_before_editing_and_alternative_definitions_are_complete() {
    let text = source::skeleton("Radiométric accuracy", &[]).unwrap();
    let doc = source::parse(&text).unwrap();
    assert_eq!(doc.sections.len(), 5);
    assert_eq!(doc.sections[1].heading, "1 Definition");
    assert_eq!(source::slug("Radiométric accuracy"), "radiometric_accuracy");
    let next = source::apply_edit(&text, Edit::AddDefinition).unwrap();
    let doc = source::parse(&next).unwrap();
    assert_eq!(doc.sections.len(), 9);
    assert_eq!(doc.sections[5].heading, "2 Definition");
    assert!(next.contains("___\n\n## 2 Definition\n\n### Notes\n\n### Examples\n\n### Sources"));
}
#[test]
fn durable_revisions_reject_stale_writers_and_isolate_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.sqlite3");
    let catalogue = bundled().unwrap();
    let term = "docs/terms/accuracy.md";
    let mut first = Store::open(&path, &catalogue).unwrap();
    let mut second = Store::open(&path, &catalogue).unwrap();
    let s = first.create_session("WGCV".into()).unwrap();
    let other = first.create_session("Independent review".into()).unwrap();
    let edit = first
        .save_edit(&s.id, term, 0, section_edit(1, "Durably saved definition."))
        .unwrap();
    assert_eq!(edit.revision, 1);
    let stale = second
        .save_edit(&s.id, term, 0, section_edit(1, "Stale overwrite"))
        .unwrap_err();
    assert_eq!(stale.code, "stale_revision");
    assert!(
        !second
            .open_term(&other.id, term)
            .unwrap()
            .document
            .source
            .contains("Durably saved")
    );
    drop(first);
    drop(second);
    let recovered = Store::open(&path, &catalogue).unwrap();
    assert!(
        recovered
            .open_term(&s.id, term)
            .unwrap()
            .document
            .source
            .contains("Durably saved definition.")
    );
    assert_eq!(recovered.changes(&s.id, false).unwrap().len(), 1);
}
#[test]
fn new_terms_and_portable_sessions_preserve_identity_without_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let catalogue = bundled().unwrap();
    let mut first = Store::open(&dir.path().join("a.db"), &catalogue).unwrap();
    let s = first.create_session("New terminology".into()).unwrap();
    let term = first
        .new_term(&s.id, "A new scientific term", &["WGCV".into()])
        .unwrap();
    assert!(term.is_new);
    assert_eq!(term.path, "docs/terms/a_new_scientific_term.md");
    assert!(first.new_term(&s.id, "A new scientific term", &[]).is_err());
    let export = first.export(&s.id).unwrap();
    assert!(!export.contains("access_token"));
    let mut second = Store::open(&dir.path().join("b.db"), &catalogue).unwrap();
    let imported = second.import(&export).unwrap();
    assert_eq!(s.id, imported.id);
    assert_eq!(
        second.open_term(&s.id, &term.path).unwrap().document.source,
        term.document.source
    );
    assert!(second.import(&export).is_err());
    first.revert(&s.id, &term.path, 0).unwrap();
    assert!(first.open_term(&s.id, &term.path).is_err());
}
#[test]
fn remote_changes_merge_only_when_non_overlapping() {
    let base = "first\n\nmiddle\n\nlast\n";
    assert_eq!(
        merge_source(
            base,
            "FIRST\n\nmiddle\n\nlast\n",
            Some("first\n\nmiddle\n\nLAST\n"),
            "term"
        )
        .unwrap(),
        "FIRST\n\nmiddle\n\nLAST\n"
    );
    assert_eq!(
        merge_source(base, "mine\n", Some("theirs\n"), "term")
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(
        merge_source(base, "mine", None, "term").unwrap_err().code,
        "conflict"
    );
    assert_eq!(merge_source("", "new", None, "term").unwrap(), "new");
}
#[test]
fn force_kill_after_acknowledgement_recovers_durable_draft() {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
    };
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("crash.sqlite3");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "crash_writer_child", "--nocapture"])
        .env("EO_EDITOR_CRASH_DB", &db)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    let session_id = loop {
        line.clear();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "Child exited without an acknowledgement"
        );
        if let Some(id) = line.strip_prefix("ACK:") {
            break id.trim().to_owned();
        }
    };
    child.kill().unwrap();
    child.wait().unwrap();
    let store = Store::open(&db, &bundled().unwrap()).unwrap();
    assert!(
        store
            .open_term(&session_id, "docs/terms/accuracy.md")
            .unwrap()
            .document
            .source
            .contains("Survives forced termination.")
    );
}
#[test]
#[ignore = "subprocess fixture for the forced-termination test"]
fn crash_writer_child() {
    use std::io::Write;
    let path = std::env::var_os("EO_EDITOR_CRASH_DB").expect("subprocess database");
    let mut store = Store::open(std::path::Path::new(&path), &bundled().unwrap()).unwrap();
    let session = store.create_session("Crash recovery".into()).unwrap();
    store
        .save_edit(
            &session.id,
            "docs/terms/accuracy.md",
            0,
            section_edit(1, "Survives forced termination."),
        )
        .unwrap();
    println!("ACK:{}", session.id);
    std::io::stdout().flush().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));
}
