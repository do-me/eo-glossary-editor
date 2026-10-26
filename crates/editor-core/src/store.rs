use std::{collections::BTreeMap, path::Path, time::Duration};

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Error, Result,
    model::*,
    source::{self, Document, Edit},
};

pub struct Store {
    connection: Connection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTerm {
    pub path: String,
    pub document: Document,
    pub revision: u64,
    pub is_new: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub before_page: Option<crate::review::Page>,
    pub after_page: crate::review::Page,
    pub path: String,
    pub title: String,
    pub before: String,
    pub after: String,
    pub diff: String,
    pub warnings: Vec<String>,
    pub is_new: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExport {
    pub schema_version: u32,
    pub session: Session,
    pub catalogue: Catalogue,
}

impl Store {
    pub fn open(path: &Path, bundled: &Catalogue) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 2 {
            return Err(Error::new(
                "storage_version",
                "This database was created by a newer editor. Open it with that version.",
            ));
        }
        if version == 0 {
            tx.execute_batch("CREATE TABLE snapshots (repository TEXT NOT NULL,sha TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(repository,sha));
                CREATE TABLE sessions (id TEXT PRIMARY KEY,data TEXT NOT NULL,updated TEXT NOT NULL);
                CREATE TABLE preferences (key TEXT PRIMARY KEY,data TEXT NOT NULL);
                CREATE TABLE revisions (session TEXT NOT NULL,path TEXT NOT NULL,revision INTEGER NOT NULL,source TEXT NOT NULL,created TEXT NOT NULL,PRIMARY KEY(session,path,revision));")?;
        } else if version == 1 {
            tx.execute_batch("ALTER TABLE snapshots RENAME TO snapshots_v1;
                CREATE TABLE snapshots (repository TEXT NOT NULL,sha TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(repository,sha));
                INSERT INTO snapshots SELECT json_extract(data,'$.repository'),sha,data FROM snapshots_v1;
                INSERT OR REPLACE INTO preferences(key,data) SELECT 'repository',json_extract(data,'$.repository') FROM snapshots_v1 WHERE sha=(SELECT data FROM preferences WHERE key='catalogue');
                DROP TABLE snapshots_v1;")?;
        }
        tx.execute_batch("PRAGMA user_version=2;")?;
        tx.commit()?;
        let mut store = Self { connection };
        if store.preference("catalogue")?.is_none() {
            store.install_catalogue(bundled)?;
        }
        Ok(store)
    }

    pub fn install_catalogue(&mut self, catalogue: &Catalogue) -> Result<()> {
        validate_catalogue(catalogue)?;
        let tx = self.connection.transaction()?;
        cache_catalogue(&tx, catalogue)?;
        select_catalogue(&tx, catalogue)?;
        tx.commit()?;
        Ok(())
    }

    pub fn catalogue(&self, sha: Option<&str>) -> Result<Catalogue> {
        let current = self
            .preference("catalogue")?
            .ok_or_else(|| Error::new("catalogue", "No glossary snapshot is available."))?;
        let repository = self
            .preference("repository")?
            .unwrap_or_else(crate::repository::default_repository);
        self.catalogue_for(&repository, sha.unwrap_or(&current))
    }

    pub fn catalogue_for(&self, repository: &str, sha: &str) -> Result<Catalogue> {
        let data: String = self.connection.query_row(
            "SELECT data FROM snapshots WHERE repository=?1 AND sha=?2",
            (repository, sha),
            |r| r.get(0),
        )?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn latest_catalogue(&self, repository: &str) -> Result<Catalogue> {
        let data:String=self.connection.query_row("SELECT data FROM snapshots WHERE repository=?1 ORDER BY json_extract(data,'$.fetchedAt') DESC LIMIT 1",[repository],|r|r.get(0))?;
        Ok(serde_json::from_str(&data)?)
    }
    pub fn switch_repository(&mut self, catalogue: &Catalogue) -> Result<Session> {
        validate_catalogue(catalogue)?;
        let session = Session::new(format!("{} review", catalogue.repository), catalogue);
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        cache_catalogue(&tx, catalogue)?;
        select_catalogue(&tx, catalogue)?;
        tx.execute(
            "INSERT INTO sessions(id,data,updated) VALUES (?1,?2,?3)",
            (
                &session.id,
                serde_json::to_string(&session)?,
                &session.updated_at,
            ),
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO preferences(key,data) VALUES ('active_session',?1)",
            [&session.id],
        )?;
        tx.commit()?;
        Ok(session)
    }

    pub fn preferences(&self) -> Result<Preferences> {
        self.preference("ui_preferences")?
            .map(|s| serde_json::from_str(&s).map_err(Error::from))
            .transpose()
            .map(|p| p.unwrap_or_default())
    }
    pub fn save_preferences(&self, preferences: Preferences) -> Result<Preferences> {
        if !(75..=200).contains(&preferences.zoom) {
            return Err(Error::new("zoom", "Zoom must be between 75% and 200%."));
        }
        self.set_preference("ui_preferences", &serde_json::to_string(&preferences)?)?;
        Ok(preferences)
    }
    pub fn cache_remote(&mut self, catalogue: &Catalogue) -> Result<()> {
        validate_catalogue(catalogue)?;
        let current = self.catalogue(None)?;
        let tx = self.connection.transaction()?;
        cache_catalogue(&tx, catalogue)?;
        if current.repository == catalogue.repository {
            select_catalogue(&tx, catalogue)?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn apply_updates(&mut self, id: &str, sha: &str) -> Result<crate::sync::Report> {
        let initial = self.session(id)?;
        let previous = self.catalogue_for(&initial.repository, &initial.base_sha)?;
        let latest = self.catalogue_for(&initial.repository, sha)?;
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut session = read_session(&tx, id)?;
        if session.base_sha != initial.base_sha {
            return Err(Error::new(
                "stale_revision",
                "The session changed while applying GitHub updates. Check again.",
            ));
        }
        if session.operation.is_some() {
            return crate::sync::report(&session, now(), 0, 0);
        }
        if let Some(data) = self_pr_check(&tx, id)?
            && let Some(link) = &mut session.submission
            && data["number"].as_u64() == Some(link.number)
            && let Some(state) = data["state"]
                .as_str()
                .filter(|s| matches!(*s, "open" | "closed" | "merged"))
        {
            link.state = state.into();
        }
        let old = session.terms.clone();
        let (updated, merged) = crate::sync::reconcile(&mut session, &previous, &latest)?;
        for (path, draft) in &old {
            if session.terms.get(path).map(|d| &d.source) != Some(&draft.source) {
                record_revision(&tx, id, path, draft.revision, &draft.source)?;
            }
        }
        session.updated_at = now();
        persist(&tx, &session)?;
        tx.commit()?;
        crate::sync::report(&session, now(), updated, merged)
    }
    pub fn sync_report(&self, id: &str) -> Result<crate::sync::Report> {
        crate::sync::report(&self.session(id)?, now(), 0, 0)
    }
    pub fn resolve_update(
        &mut self,
        id: &str,
        path: &str,
        revision: u64,
        sha: &str,
        choice: crate::sync::Choice,
    ) -> Result<crate::sync::Report> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut session = read_session(&tx, id)?;
        if session.operation.is_some() {
            return Err(Error::new("busy", "Recover the pending upload first."));
        }
        let conflict = session
            .conflicts
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new("sync", "This conflict has already been resolved."))?;
        let draft = session
            .terms
            .get_mut(path)
            .ok_or_else(|| Error::new("sync", "The local draft is missing."))?;
        if draft.revision != revision || conflict.github_sha != sha {
            return Err(Error::new(
                "stale_revision",
                "The term changed. Review its current differences before choosing a version.",
            ));
        }
        record_revision(&tx, id, path, draft.revision, &draft.source)?;
        if let crate::sync::Choice::Github = choice {
            if let Some(remote) = conflict.github {
                draft.source = remote;
                draft.is_new = false;
            } else {
                if draft.submitted.is_some() {
                    return Err(Error::new(
                        "sync",
                        "GitHub removed this term. Remove it from the open PR on GitHub, or keep your version to propose restoring it.",
                    ));
                }
                session.terms.remove(path);
                session.workspace.tabs.retain(|p| p != path);
                session.workspace.pinned.retain(|p| p != path);
                if session.workspace.active.as_deref() == Some(path) {
                    session.workspace.active = session.workspace.tabs.last().cloned();
                }
            }
        }
        if let Some(draft) = session.terms.get_mut(path) {
            draft.revision += 1;
        }
        session.conflicts.remove(path);
        session.updated_at = now();
        persist(&tx, &session)?;
        tx.commit()?;
        crate::sync::report(&session, now(), 0, 0)
    }
    pub fn preference(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row("SELECT data FROM preferences WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_preference(&self, key: &str, data: &str) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO preferences(key,data) VALUES (?1,?2)",
            (key, data),
        )?;
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self
            .connection
            .prepare("SELECT data FROM sessions ORDER BY updated DESC")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| {
            let session: Session = serde_json::from_str(&r?)?;
            Ok(session.summary())
        })
        .collect()
    }

    pub fn session(&self, id: &str) -> Result<Session> {
        read_session(&self.connection, id)
    }

    pub fn create_session(&mut self, name: String) -> Result<Session> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err(Error::new(
                "name",
                "Enter a session name of 1–200 characters.",
            ));
        }
        let session = Session::new(name.to_owned(), &self.catalogue(None)?);
        self.insert_session(&session)?;
        self.set_preference("active_session", &session.id)?;
        Ok(session)
    }

    fn insert_session(&mut self, session: &Session) -> Result<()> {
        self.connection.execute(
            "INSERT INTO sessions(id,data,updated) VALUES (?1,?2,?3)",
            (
                &session.id,
                serde_json::to_string(session)?,
                &session.updated_at,
            ),
        )?;
        Ok(())
    }

    /// Every process/CLI writer re-reads inside an IMMEDIATE transaction.
    pub fn mutate<T>(
        &mut self,
        id: &str,
        apply: impl FnOnce(&mut Session) -> Result<T>,
    ) -> Result<T> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut session = read_session(&tx, id)?;
        let value = apply(&mut session)?;
        session.updated_at = now();
        persist(&tx, &session)?;
        tx.commit()?;
        Ok(value)
    }

    pub fn catalogue_terms(&self, session_id: Option<&str>) -> Result<Vec<TermSummary>> {
        let session = session_id.map(|id| self.session(id)).transpose()?;
        let catalogue = match &session {
            Some(s) => self.catalogue_for(&s.repository, &s.base_sha)?,
            None => self.catalogue(None)?,
        };
        let mut sources = catalogue.files;
        if let Some(session) = &session {
            for (path, draft) in &session.terms {
                sources.insert(path.clone(), draft.source.clone());
            }
        }
        let mut result: Vec<TermSummary> = sources
            .into_iter()
            .filter_map(|(path, text)| {
                let draft = session.as_ref().and_then(|s| s.terms.get(&path));
                source::summary(
                    &path,
                    &text,
                    draft.is_some_and(Draft::changed),
                    draft.is_some_and(|d| d.pending() || (d.is_new && d.submitted.is_none())),
                    draft.is_some_and(|d| d.is_new),
                )
                .ok()
            })
            .collect();
        result.sort_by_key(|t| t.title.to_lowercase());
        Ok(result)
    }

    pub fn open_term(&self, session_id: &str, path: &str) -> Result<OpenTerm> {
        source::validate_path(path)?;
        let session = self.session(session_id)?;
        if let Some(draft) = session.terms.get(path) {
            return Ok(OpenTerm {
                path: path.to_owned(),
                document: source::parse(&draft.source)?,
                revision: draft.revision,
                is_new: draft.is_new,
            });
        }
        let catalogue = self.catalogue_for(&session.repository, &session.base_sha)?;
        let text = catalogue.files.get(path).ok_or_else(|| {
            Error::new("not_found", "The term is not in this session's snapshot.")
        })?;
        Ok(OpenTerm {
            path: path.to_owned(),
            document: source::parse(text)?,
            revision: 0,
            is_new: false,
        })
    }

    pub fn save_edit(
        &mut self,
        session_id: &str,
        path: &str,
        expected: u64,
        edit: Edit,
    ) -> Result<OpenTerm> {
        source::validate_path(path)?;
        let session = self.session(session_id)?;
        let catalogue = self.catalogue_for(&session.repository, &session.base_sha)?;
        let original = catalogue.files.get(path).cloned();
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut session = read_session(&tx, session_id)?;
        // Finish already queued local saves even if a startup check just discovered the PR closed. Publication still refuses closed PRs.
        if !session.terms.contains_key(path) {
            let starting = original.ok_or_else(|| {
                Error::new("not_found", "Term is not present in the session snapshot.")
            })?;
            session.terms.insert(
                path.to_owned(),
                Draft {
                    source: starting.clone(),
                    starting,
                    revision: 0,
                    submitted: None,
                    is_new: false,
                },
            );
        }
        let draft = session
            .terms
            .get_mut(path)
            .ok_or_else(|| Error::new("not_found", "Draft is missing."))?;
        if draft.revision != expected {
            return Err(Error::new(
                "stale_revision",
                "This term was changed elsewhere. Your pending edit is retained; reload before continuing.",
            ));
        }
        let result = source::apply_edit(&draft.source, edit)?;
        if result != draft.source {
            draft.revision += 1;
            draft.source = result;
            tx.execute("INSERT INTO revisions(session,path,revision,source,created) VALUES (?1,?2,?3,?4,?5)", (session_id,path,i64::try_from(draft.revision).map_err(|_|Error::new("revision","Revision limit reached."))?,&draft.source,now()))?;
            tx.execute(
                "DELETE FROM revisions WHERE session=?1 AND path=?2 AND revision < ?3",
                (
                    session_id,
                    path,
                    i64::try_from(draft.revision.saturating_sub(50)).unwrap_or(i64::MAX),
                ),
            )?;
        }
        let opened = OpenTerm {
            path: path.to_owned(),
            document: source::parse(&draft.source)?,
            revision: draft.revision,
            is_new: draft.is_new,
        };
        session.updated_at = now();
        persist(&tx, &session)?;
        tx.commit()?;
        Ok(opened)
    }

    pub fn new_term(&mut self, session_id: &str, title: &str, tags: &[String]) -> Result<OpenTerm> {
        let path = format!("docs/terms/{}.md", source::slug(title));
        source::validate_path(&path)?;
        let source = source::skeleton(title, tags)?;
        let session = self.session(session_id)?;
        if self
            .catalogue_for(&session.repository, &session.base_sha)?
            .files
            .contains_key(&path)
        {
            return Err(Error::new(
                "duplicate",
                "A term with this filename already exists.",
            ));
        }
        self.mutate(session_id, |session| {
            ensure_editable(session)?;
            if session.terms.contains_key(&path) {
                return Err(Error::new(
                    "duplicate",
                    "This term is already in the session.",
                ));
            }
            session.terms.insert(
                path.clone(),
                Draft {
                    starting: String::new(),
                    source: source.clone(),
                    submitted: None,
                    revision: 0,
                    is_new: true,
                },
            );
            Ok(())
        })?;
        self.open_term(session_id, &path)
    }

    pub fn save_workspace(&mut self, id: &str, workspace: Workspace) -> Result<()> {
        if workspace.tabs.len() > 100 {
            return Err(Error::new(
                "workspace",
                "Please keep fewer than 100 open tabs.",
            ));
        }
        for path in &workspace.tabs {
            source::validate_path(path)?;
        }
        self.mutate(id, |s| {
            s.workspace = workspace;
            Ok(())
        })?;
        self.set_preference("active_session", id)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<()> {
        if name.trim().is_empty() || name.len() > 200 {
            return Err(Error::new(
                "name",
                "Enter a session name of 1–200 characters.",
            ));
        }
        self.mutate(id, |s| {
            s.name = name.trim().to_owned();
            Ok(())
        })
    }

    pub fn revert(&mut self, id: &str, path: &str, expected: u64) -> Result<()> {
        self.mutate(id, |s| {
            ensure_editable(s)?;
            let d = s.terms.get_mut(path).ok_or_else(|| {
                Error::new("not_found", "There are no saved changes for this term.")
            })?;
            if d.revision != expected {
                return Err(Error::new(
                    "stale_revision",
                    "The draft changed. Reload before reverting.",
                ));
            }
            if d.is_new && d.submitted.is_none() {
                s.terms.remove(path);
            } else {
                d.source = d.submitted.clone().unwrap_or_else(|| d.starting.clone());
                d.revision += 1;
            }
            Ok(())
        })
    }

    pub fn changes(&self, id: &str, all: bool) -> Result<Vec<Change>> {
        let session = self.session(id)?;
        session
            .terms
            .iter()
            .filter_map(|(path, draft)| {
                let before = if all {
                    &draft.starting
                } else {
                    draft.submitted.as_ref().unwrap_or(&draft.starting)
                };
                if before == &draft.source {
                    return None;
                }
                Some((|| {
                    let doc = source::parse(&draft.source)?;
                    let diff = similar::TextDiff::from_lines(before, &draft.source)
                        .unified_diff()
                        .context_radius(3)
                        .header(path, path)
                        .to_string();
                    Ok(Change {
                        before_page: if before.is_empty() {
                            None
                        } else {
                            Some(crate::review::page(before)?)
                        },
                        after_page: crate::review::page(&draft.source)?,
                        path: path.clone(),
                        title: doc.title,
                        before: before.clone(),
                        after: draft.source.clone(),
                        diff,
                        warnings: doc.warnings,
                        is_new: draft.is_new,
                    })
                })())
            })
            .collect()
    }

    /// Start a fresh contribution with only the remaining local work. The original session/journal stays recoverable.
    pub fn follow_up(&mut self, id: &str) -> Result<Session> {
        let previous = self.session(id)?;
        let catalogue = self.latest_catalogue(&previous.repository)?;
        let mut next = Session::new(format!("{} — follow-up", previous.name), &catalogue);
        for (path, draft) in &previous.terms {
            if !draft.pending() {
                continue;
            }
            let baseline = draft.submitted.as_deref().unwrap_or(&draft.starting);
            let current = catalogue.files.get(path);
            let merged = crate::github::merge_source(
                baseline,
                &draft.source,
                current.map(String::as_str),
                path,
            )?;
            next.terms.insert(
                path.clone(),
                Draft {
                    starting: current.cloned().unwrap_or_default(),
                    source: merged,
                    submitted: None,
                    revision: 0,
                    is_new: current.is_none(),
                },
            );
        }
        self.insert_session(&next)?;
        Ok(next)
    }

    pub fn export(&self, id: &str) -> Result<String> {
        let session = self.session(id)?;
        if session.operation.is_some() {
            return Err(Error::new(
                "busy",
                "Finish or recover the interrupted submission before exporting.",
            ));
        }
        let catalogue = self.catalogue_for(&session.repository, &session.base_sha)?;
        Ok(serde_json::to_string_pretty(&SessionExport {
            schema_version: 1,
            session,
            catalogue,
        })?)
    }

    pub fn import(&mut self, text: &str) -> Result<Session> {
        if text.len() > 25 * 1024 * 1024 {
            return Err(Error::new(
                "size",
                "Session files must be smaller than 25 MiB.",
            ));
        }
        let mut data: SessionExport = serde_json::from_str(text)?;
        if data.schema_version != 1
            || data.session.schema_version != 1
            || data.session.repository != data.catalogue.repository
            || data.session.base_sha != data.catalogue.sha
            || data.session.operation.is_some()
        {
            return Err(Error::new(
                "format",
                "Unsupported or inconsistent session file.",
            ));
        }
        uuid::Uuid::parse_str(&data.session.id)
            .map_err(|_| Error::new("format", "Session identifier is invalid."))?;
        validate_catalogue(&data.catalogue)?;
        for (path, draft) in &data.session.terms {
            source::validate_path(path)?;
            source::parse(&draft.source)?;
            if !draft.is_new && data.catalogue.files.get(path) != Some(&draft.starting) {
                return Err(Error::new(
                    "format",
                    "A term's starting source does not match the exported snapshot.",
                ));
            }
            if draft.is_new && !draft.starting.is_empty() {
                return Err(Error::new(
                    "format",
                    "A new term must have an empty starting source.",
                ));
            }
        }
        if let Some(link) = &data.session.submission {
            validate_link(&data.session.repository, link)?;
            if let Some(existing) = self.find_pr(&data.session.repository, link.number)? {
                return Ok(existing);
            }
        }
        if self.session(&data.session.id).is_ok() {
            return Err(Error::new(
                "duplicate",
                "This session already exists locally. Open it from Sessions.",
            ));
        }
        // Importing a snapshot must not move the user's current catalogue backwards.
        self.connection.execute(
            "INSERT OR IGNORE INTO snapshots(repository,sha,data) VALUES (?1,?2,?3)",
            (
                &data.catalogue.repository,
                &data.catalogue.sha,
                serde_json::to_string(&data.catalogue)?,
            ),
        )?;
        data.session.updated_at = now();
        self.insert_session(&data.session)?;
        Ok(data.session)
    }

    pub fn find_pr(&self, repository: &str, number: u64) -> Result<Option<Session>> {
        for summary in self.sessions()? {
            if summary.repository == repository
                && summary.submission.is_some_and(|s| s.number == number)
            {
                return self.session(&summary.id).map(Some);
            }
        }
        Ok(None)
    }

    pub fn import_pr(&mut self, base: Catalogue, session: Session) -> Result<Session> {
        if session.repository != base.repository || session.base_sha != base.sha {
            return Err(Error::new(
                "repository",
                "Imported session and snapshot repositories do not match.",
            ));
        }
        if let Some(link) = &session.submission {
            validate_link(&session.repository, link)?;
        }
        if let Some(link) = &session.submission
            && let Some(existing) = self.find_pr(&session.repository, link.number)?
        {
            return Ok(existing);
        }
        validate_catalogue(&base)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO snapshots(repository,sha,data) VALUES (?1,?2,?3)",
            (&base.repository, &base.sha, serde_json::to_string(&base)?),
        )?;
        self.insert_session(&session)?;
        Ok(session)
    }

    pub fn finish_submission(
        &mut self,
        id: &str,
        operation: &Operation,
        link: Submission,
    ) -> Result<()> {
        self.mutate(id, |s| {
            validate_link(&s.repository, &link)?;
            if operation.repository != s.repository {
                return Err(Error::new(
                    "repository",
                    "Submission repository does not match the session.",
                ));
            }
            if s.operation.as_ref().map(|o| &o.id) != Some(&operation.id) {
                return Err(Error::new(
                    "operation",
                    "Submission operation no longer matches this session.",
                ));
            }
            for (path, content) in &operation.files {
                if let Some(draft) = s.terms.get_mut(path) {
                    if operation.original_files.get(path) == Some(&draft.source)
                        && draft.source != *content
                    {
                        draft.source = content.clone();
                        draft.revision += 1;
                    }
                    draft.submitted = Some(content.clone());
                }
            }
            s.submission = Some(link);
            s.operation = None;
            Ok(())
        })
    }

    pub fn bootstrap(&self) -> Result<Value> {
        let catalogue = self.catalogue(None)?;
        Ok(
            serde_json::json!({ "sessions": self.sessions()?, "activeSession": self.preference("active_session")?,
            "catalogue": { "repository":catalogue.repository,"sha": catalogue.sha, "fetchedAt": catalogue.fetched_at, "count": catalogue.files.len(), "branch": catalogue.branch },
            "preferences":self.preferences()?,"defaultRepository":UPSTREAM,"teams": TEAM_TAGS, "classes": CLASSES, "statuses": STATUSES }),
        )
    }
}

fn read_session(connection: &Connection, id: &str) -> Result<Session> {
    let data: Option<String> = connection
        .query_row("SELECT data FROM sessions WHERE id=?1", [id], |r| r.get(0))
        .optional()?;
    let data =
        data.ok_or_else(|| Error::new("not_found", "This session is not available locally."))?;
    Ok(serde_json::from_str(&data)?)
}

fn persist(tx: &Transaction<'_>, session: &Session) -> Result<()> {
    tx.execute(
        "UPDATE sessions SET data=?1,updated=?2 WHERE id=?3",
        (
            serde_json::to_string(session)?,
            &session.updated_at,
            &session.id,
        ),
    )?;
    Ok(())
}

pub fn ensure_editable(session: &Session) -> Result<()> {
    if session
        .submission
        .as_ref()
        .is_some_and(|s| s.state != "open")
    {
        return Err(Error::new(
            "closed",
            "This contribution is closed. Start a new session to continue.",
        ));
    }
    Ok(())
}

pub fn validate_catalogue(catalogue: &Catalogue) -> Result<()> {
    if !crate::repository::valid(&catalogue.repository)
        || catalogue.sha.len() != 40
        || !catalogue.sha.bytes().all(|b| b.is_ascii_hexdigit())
        || catalogue.files.is_empty()
        || catalogue.files.len() > 20_000
    {
        return Err(Error::new(
            "catalogue",
            "Invalid or unsupported glossary snapshot.",
        ));
    }
    for (path, text) in &catalogue.files {
        source::validate_path(path)?;
        source::parse(text)?;
    }
    Ok(())
}

pub fn validate_link(repository: &str, link: &Submission) -> Result<()> {
    let expected = format!("https://github.com/{repository}/pull/{}", link.number);
    if link.url != expected
        || !valid_repo(&link.head_repo)
        || link.head_branch.is_empty()
        || link.head_branch.len() > 200
        || link.head_branch.contains("..")
        || !link
            .head_branch
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.'))
    {
        return Err(Error::new(
            "submission",
            "Invalid submission link. Re-import it from GitHub.",
        ));
    }
    Ok(())
}

pub fn valid_repo(repo: &str) -> bool {
    crate::repository::valid(repo)
}

pub fn pending_files(session: &Session) -> BTreeMap<String, String> {
    session
        .terms
        .iter()
        .filter(|(_, d)| d.pending() || (d.is_new && d.submitted.is_none()))
        .map(|(p, d)| (p.clone(), d.source.clone()))
        .collect()
}

fn cache_catalogue(tx: &Transaction<'_>, catalogue: &Catalogue) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO snapshots(repository,sha,data) VALUES (?1,?2,?3)",
        (
            &catalogue.repository,
            &catalogue.sha,
            serde_json::to_string(catalogue)?,
        ),
    )?;
    Ok(())
}
fn select_catalogue(tx: &Transaction<'_>, catalogue: &Catalogue) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO preferences(key,data) VALUES ('catalogue',?1)",
        [&catalogue.sha],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO preferences(key,data) VALUES ('repository',?1)",
        [&catalogue.repository],
    )?;
    Ok(())
}

fn record_revision(
    tx: &Transaction<'_>,
    id: &str,
    path: &str,
    revision: u64,
    source: &str,
) -> Result<()> {
    tx.execute("INSERT OR IGNORE INTO revisions(session,path,revision,source,created) VALUES (?1,?2,?3,?4,?5)",(id,path,i64::try_from(revision).map_err(|_|Error::new("revision","Revision limit reached."))?,source,now()))?;
    Ok(())
}

fn self_pr_check(tx: &Transaction<'_>, id: &str) -> Result<Option<serde_json::Value>> {
    let raw: Option<String> = tx
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [format!("checked_pr:{id}")],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|s| serde_json::from_str(&s).map_err(Error::from))
        .transpose()
}
