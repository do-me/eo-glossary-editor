use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const UPSTREAM: &str = "ceos-org/eo-glossary";
pub const TEAM_TAGS: &[&str] = &[
    "WGCV",
    "WGISS",
    "WGClimate",
    "WGCapD",
    "WGDisasters",
    "AC-VC",
    "COAST-VC",
    "B-VC",
    "LSI-VC",
    "OCR-VC",
    "OST-VC",
    "OSVW-VC",
    "P-VC",
    "SST-VC",
];
pub const CLASSES: &[&str] = &["base", "core", "controversial", "high-impact"];
pub const STATUSES: &[&str] = &[
    "to be defined",
    "to be discussed",
    "to be approved",
    "approved",
];

pub fn now() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    pub sha: String,
    pub branch: String,
    pub fetched_at: String,
    pub repository: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workspace {
    pub tabs: Vec<String>,
    pub pinned: Vec<String>,
    pub active: Option<String>,
    pub query: String,
    pub teams: Vec<String>,
    pub classes: Vec<String>,
    pub statuses: Vec<String>,
    pub changed_only: bool,
    pub scroll: BTreeMap<String, f64>,
    pub view: String,
    pub filters_open: bool,
    pub view_scroll: BTreeMap<String, f64>,
    pub sidebar_scroll: f64,
    pub tabs_scroll: f64,
    pub term_ui: BTreeMap<String, TermUi>,
    pub review_mode: String,
    pub all_changes: bool,
    pub contribution_title: String,
    pub contribution_body: String,
    pub dialog: Option<String>,
    pub new_term_title: String,
    pub new_term_teams: Vec<String>,
    pub session_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TermUi {
    pub details: bool,
    pub source: bool,
    pub source_sections: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub zoom: u16,
}
impl Default for Preferences {
    fn default() -> Self {
        Self { zoom: 100 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub starting: String,
    pub source: String,
    pub submitted: Option<String>,
    pub revision: u64,
    pub is_new: bool,
}

impl Draft {
    pub fn pending(&self) -> bool {
        self.source != self.submitted.as_deref().unwrap_or(&self.starting)
    }
    pub fn changed(&self) -> bool {
        self.is_new || self.source != self.starting
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub number: u64,
    pub url: String,
    pub head_repo: String,
    pub head_branch: String,
    pub head_sha: String,
    pub account_id: u64,
    pub state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default = "crate::repository::default_repository")]
    pub repository: String,
    #[serde(default = "crate::repository::default_branch")]
    pub base_branch: String,
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub original_files: BTreeMap<String, String>,
    pub files: BTreeMap<String, String>,
    pub head_repo: String,
    pub head_branch: String,
    pub parent: String,
    pub commit: Option<String>,
    pub account_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub repository: String,
    pub base_sha: String,
    pub base_branch: String,
    pub created_at: String,
    pub updated_at: String,
    pub terms: BTreeMap<String, Draft>,
    pub workspace: Workspace,
    pub submission: Option<Submission>,
    pub operation: Option<Operation>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub conflicts: crate::sync::Conflicts,
}

impl Session {
    pub fn new(name: String, catalogue: &Catalogue) -> Self {
        let time = now();
        Self {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            name,
            repository: catalogue.repository.clone(),
            base_sha: catalogue.sha.clone(),
            base_branch: catalogue.branch.clone(),
            created_at: time.clone(),
            updated_at: time,
            terms: BTreeMap::new(),
            workspace: Workspace::default(),
            submission: None,
            operation: None,
            archived: false,
            conflicts: Default::default(),
        }
    }
    pub fn summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            repository: self.repository.clone(),
            updated_at: self.updated_at.clone(),
            changed: self.terms.values().filter(|d| d.changed()).count(),
            pending: self
                .terms
                .values()
                .filter(|d| d.pending() || (d.is_new && d.submitted.is_none()))
                .count(),
            submission: self.submission.clone(),
            archived: self.archived,
            interrupted: self.operation.is_some(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub repository: String,
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub changed: usize,
    pub pending: usize,
    pub submission: Option<Submission>,
    pub archived: bool,
    pub interrupted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TermSummary {
    pub path: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub search_text: String,
    pub changed: bool,
    pub pending: bool,
    pub is_new: bool,
}
