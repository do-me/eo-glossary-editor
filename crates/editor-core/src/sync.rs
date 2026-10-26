//! Reconcile approved main snapshots with durable drafts. No GitHub writes occur here.
use crate::{Error, Result, model::*, review, source, store::Change};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub base: String,
    pub github: Option<String>,
    pub github_sha: String,
    pub detected_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictView {
    pub path: String,
    pub title: String,
    pub revision: u64,
    pub github_sha: String,
    pub deleted: bool,
    pub local_diff: String,
    pub github_diff: String,
    pub comparison: Change,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub repository: String,
    pub sha: String,
    pub branch: String,
    pub checked_at: String,
    pub updated: usize,
    pub merged: usize,
    pub deferred: bool,
    pub conflicts: Vec<ConflictView>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Github,
    Local,
}
pub fn reconcile(
    session: &mut Session,
    previous: &Catalogue,
    latest: &Catalogue,
) -> Result<(usize, usize)> {
    if session.repository != latest.repository || previous.repository != session.repository {
        return Err(Error::new(
            "repository",
            "The update belongs to a different repository.",
        ));
    }
    if session.operation.is_some() {
        return Err(Error::new(
            "busy",
            "Recover the interrupted submission before applying GitHub updates.",
        ));
    }
    let mut updated = 0;
    let mut merged = 0;
    if session.base_sha == latest.sha {
        return Ok((updated, merged));
    }
    for (path, draft) in &mut session.terms {
        let base = previous.files.get(path).map(String::as_str).unwrap_or("");
        let remote = latest.files.get(path);
        // An unresolved choice remains unresolved across additional updates. Re-evaluate against its original common ancestor.
        let common = session
            .conflicts
            .get(path)
            .map(|c| c.base.as_str())
            .unwrap_or(base);
        let combined = if remote.is_none() && draft.source == common && draft.submitted.is_none() {
            Ok(String::new())
        } else {
            crate::github::merge_source(common, &draft.source, remote.map(String::as_str), path)
        };
        let before = draft.source.clone();
        match combined {
            Ok(text) => {
                if !text.is_empty() {
                    source::parse(&text)?;
                }
                if text != draft.source {
                    draft.source = text;
                    updated += 1;
                    if before != common {
                        merged += 1;
                    }
                }
                session.conflicts.remove(path);
            }
            Err(error) if error.code == "conflict" => {
                session.conflicts.insert(
                    path.clone(),
                    Conflict {
                        base: common.to_owned(),
                        github: remote.cloned(),
                        github_sha: latest.sha.clone(),
                        detected_at: now(),
                    },
                );
            }
            Err(error) => return Err(error),
        }
        draft.starting = remote.cloned().unwrap_or_default();
        draft.is_new = remote.is_none();
        draft.revision = draft
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new("revision", "Revision limit reached."))?;
    }
    // A clean local copy of a removed upstream term can disappear without a conflict.
    let removed: Vec<_> = session
        .terms
        .iter()
        .filter(|(_, d)| d.source.is_empty() && d.submitted.is_none())
        .map(|(p, _)| p.clone())
        .collect();
    for path in removed {
        session.terms.remove(&path);
        session.workspace.tabs.retain(|p| p != &path);
    }
    session.base_sha = latest.sha.clone();
    session.base_branch = latest.branch.clone();
    let exists =
        |path: &String| latest.files.contains_key(path) || session.terms.contains_key(path);
    session.workspace.tabs.retain(exists);
    session.workspace.pinned.retain(exists);
    if session
        .workspace
        .active
        .as_ref()
        .is_some_and(|p| !exists(p))
    {
        session.workspace.active = session.workspace.tabs.last().cloned();
    }
    Ok((updated, merged))
}
pub fn report(
    session: &Session,
    checked_at: String,
    updated: usize,
    merged: usize,
) -> Result<Report> {
    let mut conflicts = Vec::new();
    for (path, conflict) in &session.conflicts {
        let draft = session
            .terms
            .get(path)
            .ok_or_else(|| Error::new("sync", "The conflicting draft is missing."))?;
        let after_page = review::page(&draft.source)?;
        let title = after_page.title.clone();
        let github = conflict.github.clone().unwrap_or_default();
        conflicts.push(ConflictView {
            path: path.clone(),
            title: title.clone(),
            revision: draft.revision,
            github_sha: conflict.github_sha.clone(),
            deleted: conflict.github.is_none(),
            local_diff: diff(&conflict.base, &draft.source, path),
            github_diff: diff(&conflict.base, &github, path),
            comparison: Change {
                path: path.clone(),
                title,
                before: github.clone(),
                after: draft.source.clone(),
                before_page: if github.is_empty() {
                    None
                } else {
                    Some(review::page(&github)?)
                },
                after_page,
                diff: diff(&github, &draft.source, path),
                warnings: Vec::new(),
                is_new: draft.is_new,
            },
        });
    }
    Ok(Report {
        repository: session.repository.clone(),
        sha: session.base_sha.clone(),
        branch: session.base_branch.clone(),
        checked_at,
        updated,
        merged,
        deferred: session.operation.is_some(),
        conflicts,
    })
}
fn diff(before: &str, after: &str, path: &str) -> String {
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(path, path)
        .to_string()
}
pub type Conflicts = BTreeMap<String, Conflict>;
