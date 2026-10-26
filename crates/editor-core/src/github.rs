//! Native-only GitHub transport. Tokens are accepted as arguments and never serialized.
use crate::{
    Error, Result,
    model::*,
    source,
    store::{Store, pending_files, valid_repo},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, time::Duration};

#[derive(Clone)]
pub struct Github {
    client: Client,
    api: String,
}
impl Default for Github {
    fn default() -> Self {
        Self::new("https://api.github.com")
    }
}
impl Github {
    pub fn new(api: &str) -> Self {
        Self {
            client: Client::builder()
                .user_agent("eo-glossary-editor/0.1.0")
                .timeout(Duration::from_secs(60))
                .build()
                .expect("TLS client"),
            api: api.trim_end_matches('/').to_owned(),
        }
    }
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut req = self
            .client
            .request(method, format!("{}{path}", self.api))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req.send().await.map_err(network)?;
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let data: Value = response.json().await.map_err(network)?;
        if !status.is_success() {
            let code = match status.as_u16() {
                401 => "authentication",
                403 | 429 => "permission_or_rate_limit",
                404 => "not_found",
                409 | 422 => "remote_changed",
                _ => "github",
            };
            return Err(Error::new(
                code,
                format!(
                    "GitHub returned {status}: {}",
                    data["message"]
                        .as_str()
                        .unwrap_or("Request failed. Your local drafts are safe.")
                ),
            ));
        }
        Ok(data)
    }
    pub async fn account(&self, token: &str) -> Result<Account> {
        Ok(serde_json::from_value(
            self.request(Method::GET, "/user", Some(token), None)
                .await?,
        )?)
    }
    pub async fn repository_head(
        &self,
        repository: &str,
        token: Option<&str>,
    ) -> Result<(String, String, String)> {
        if !valid_repo(repository) {
            return Err(Error::new("repository", "Invalid repository."));
        }
        let repo = self
            .request(Method::GET, &format!("/repos/{repository}"), token, None)
            .await?;
        let branch = string(&repo, "default_branch")?;
        let canonical = crate::repository::parse(&string(&repo, "full_name")?)?;
        let commit = self
            .request(
                Method::GET,
                &format!("/repos/{canonical}/commits/{}", encode_reference(&branch)),
                token,
                None,
            )
            .await?;
        Ok((canonical, branch, string(&commit, "sha")?))
    }
    pub async fn download_repository(
        &self,
        repository: &str,
        token: Option<&str>,
    ) -> Result<Catalogue> {
        let (repository, branch, sha) = self.repository_head(repository, token).await?;
        let mut catalogue = self.snapshot_sha(&repository, &sha, token).await?;
        catalogue.branch = branch;
        Ok(catalogue)
    }
    pub async fn snapshot(
        &self,
        repository: &str,
        reference: &str,
        token: Option<&str>,
    ) -> Result<Catalogue> {
        let commit = self
            .request(
                Method::GET,
                &format!(
                    "/repos/{repository}/commits/{}",
                    encode_reference(reference)
                ),
                token,
                None,
            )
            .await?;
        let sha = string(&commit, "sha")?;
        {
            let mut catalogue = self.snapshot_sha(repository, &sha, token).await?;
            catalogue.branch = reference.to_owned();
            Ok(catalogue)
        }
    }
    pub async fn snapshot_sha(
        &self,
        repository: &str,
        sha: &str,
        token: Option<&str>,
    ) -> Result<Catalogue> {
        if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new("sha", "Invalid snapshot revision."));
        }
        let mut req = self
            .client
            .get(format!("{}/repos/{repository}/tarball/{sha}", self.api));
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        let mut response = req
            .send()
            .await
            .map_err(network)?
            .error_for_status()
            .map_err(network)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network)? {
            if bytes.len() + chunk.len() > 50 * 1024 * 1024 {
                return Err(Error::new("size", "Repository download is too large."));
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_slice()));
        let mut files = BTreeMap::new();
        let mut total = 0;
        for entry in archive.entries()? {
            let mut entry = entry?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let full = entry.path()?.to_string_lossy().into_owned();
            let Some((_, path)) = full.split_once('/') else {
                continue;
            };
            if !path.starts_with("docs/terms/") || !path.ends_with(".md") {
                continue;
            }
            source::validate_path(path)?;
            if entry.size() > 1024 * 1024 {
                return Err(Error::new("size", "A term exceeds the size limit."));
            }
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            total += text.len();
            if total > 25 * 1024 * 1024 {
                return Err(Error::new("size", "Glossary exceeds the size limit."));
            }
            source::parse(&text)?;
            files.insert(path.to_owned(), text);
        }
        let catalogue = Catalogue {
            sha: sha.to_owned(),
            branch: "main".into(),
            fetched_at: now(),
            repository: repository.to_owned(),
            files,
        };
        crate::store::validate_catalogue(&catalogue)?;
        Ok(catalogue)
    }
    pub async fn content(
        &self,
        repo: &str,
        path: &str,
        sha: &str,
        token: &str,
    ) -> Result<Option<String>> {
        source::validate_path(path)?;
        match self
            .request(
                Method::GET,
                &format!("/repos/{repo}/contents/{path}?ref={sha}"),
                Some(token),
                None,
            )
            .await
        {
            Ok(v) => {
                let raw = STANDARD
                    .decode(string(&v, "content")?.replace(['\r', '\n'], ""))
                    .map_err(|_| Error::new("encoding", "Invalid GitHub content encoding."))?;
                Ok(Some(String::from_utf8(raw).map_err(|_| {
                    Error::new("encoding", "The remote term is not UTF-8.")
                })?))
            }
            Err(e) if e.code == "not_found" => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub async fn authored_prs(
        &self,
        repository: &str,
        account: &Account,
        token: &str,
    ) -> Result<Value> {
        let query = format!("repo:{repository} is:pr is:open author:{}", account.login);
        let query = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        Ok(self
            .request(
                Method::GET,
                &format!("/search/issues?q={query}&per_page=100"),
                Some(token),
                None,
            )
            .await?["items"]
            .clone())
    }
    pub async fn public_pr(
        &self,
        repository: &str,
        number: u64,
        token: Option<&str>,
    ) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/repos/{repository}/pulls/{number}"),
            token,
            None,
        )
        .await
    }
    pub async fn pr(&self, repository: &str, number: u64, token: &str) -> Result<Value> {
        self.request(
            Method::GET,
            &format!("/repos/{repository}/pulls/{number}"),
            Some(token),
            None,
        )
        .await
    }
    pub async fn import_pr(
        &self,
        repository: &str,
        number: u64,
        account: &Account,
        token: &str,
    ) -> Result<(Catalogue, Session)> {
        let pr = self.pr(repository, number, token).await?;
        verify_pr(repository, &pr, account)?;
        let sha = string(&pr["base"], "sha")?;
        let base = self.snapshot_sha(repository, &sha, Some(token)).await?;
        let mut base = base;
        base.branch = string(&pr["base"], "ref")?;
        let mut session = Session::new(string(&pr, "title")?, &base);
        for page in 1..=30 {
            let files = self
                .request(
                    Method::GET,
                    &format!("/repos/{repository}/pulls/{number}/files?per_page=100&page={page}"),
                    Some(token),
                    None,
                )
                .await?;
            let files = files
                .as_array()
                .ok_or_else(|| Error::new("github", "Invalid changed-file response."))?;
            for file in files {
                let path = string(file, "filename")?;
                source::validate_path(&path).map_err(|_| {
                    Error::new(
                        "unsupported_pr",
                        "This PR changes files outside the term directory. Resume it on GitHub.",
                    )
                })?;
                if !matches!(file["status"].as_str(), Some("modified" | "added")) {
                    return Err(Error::new(
                        "unsupported_pr",
                        "Renamed or deleted files require review on GitHub.",
                    ));
                }
                let head_repo = string(&pr["head"]["repo"], "full_name")?;
                let head_sha = string(&pr["head"], "sha")?;
                let content = self
                    .content(&head_repo, &path, &head_sha, token)
                    .await?
                    .ok_or_else(|| Error::new("remote_changed", "A PR term is missing."))?;
                source::parse(&content)?;
                session.terms.insert(
                    path.clone(),
                    Draft {
                        starting: base.files.get(&path).cloned().unwrap_or_default(),
                        source: content.clone(),
                        submitted: Some(content),
                        revision: 0,
                        is_new: !base.files.contains_key(&path),
                    },
                );
            }
            if files.len() < 100 {
                break;
            }
            if page == 30 {
                return Err(Error::new("size", "This PR has too many files to import."));
            }
        }
        session.submission = Some(submission(repository, &pr, account.id)?);
        Ok((base, session))
    }
    pub async fn prepare(
        &self,
        session: &Session,
        account: &Account,
        token: &str,
        title: String,
        body: String,
    ) -> Result<Operation> {
        crate::store::ensure_editable(session)?;
        if !session.conflicts.is_empty() {
            return Err(Error::new(
                "sync_conflicts",
                "Review the conflicting GitHub updates before submitting. Choose a version for each term.",
            ));
        }
        let repository = &session.repository;
        if !valid_repo(repository) {
            return Err(Error::new("repository", "Invalid session repository."));
        }
        if title.trim().is_empty() || title.len() > 250 || body.len() > 60_000 {
            return Err(Error::new(
                "title",
                "Enter a contribution title (up to 250 characters).",
            ));
        }
        if let Some(op) = &session.operation {
            if op.repository != session.repository {
                return Err(Error::new(
                    "repository",
                    "The recorded upload belongs to a different repository.",
                ));
            }
            if op.account_id != account.id {
                return Err(Error::new(
                    "account",
                    "Sign in with the account that started this submission.",
                ));
            }
            return Ok(op.clone());
        }
        let mut files = pending_files(session);
        let original_files = files.clone();
        if files.is_empty() {
            return Err(Error::new(
                "empty",
                "There are no new local changes to submit.",
            ));
        }
        let (repo, branch, parent) = if let Some(link) = &session.submission {
            crate::store::validate_link(repository, link)?;
            if link.account_id != account.id {
                return Err(Error::new("account", "Sign in with the author of this PR."));
            }
            let pr = self.pr(repository, link.number, token).await?;
            verify_pr(repository, &pr, account)?;
            (
                string(&pr["head"]["repo"], "full_name")?,
                string(&pr["head"], "ref")?,
                string(&pr["head"], "sha")?,
            )
        } else {
            let upstream = self
                .request(
                    Method::GET,
                    &format!("/repos/{repository}"),
                    Some(token),
                    None,
                )
                .await?;
            let repo = if upstream["permissions"]["push"].as_bool() == Some(true) {
                repository.to_owned()
            } else {
                let repo = format!(
                    "{}/{}",
                    account.login,
                    repository.split('/').nth(1).unwrap_or_default()
                );
                match self
                    .request(Method::GET, &format!("/repos/{repo}"), Some(token), None)
                    .await
                {
                    Ok(v) if same_network(&upstream, &v, repository) => {}
                    Ok(_) => {
                        return Err(Error::new(
                            "fork",
                            "A repository with that name already exists on your account outside this fork network.",
                        ));
                    }
                    Err(e) if e.code == "not_found" => {
                        self.request(
                            Method::POST,
                            &format!("/repos/{repository}/forks"),
                            Some(token),
                            Some(json!({"default_branch_only":true})),
                        )
                        .await?;
                    }
                    Err(e) => return Err(e),
                }
                repo
            };
            let main = self
                .request(
                    Method::GET,
                    &format!(
                        "/repos/{repository}/commits/{}",
                        encode_reference(&session.base_branch)
                    ),
                    Some(token),
                    None,
                )
                .await?;
            (
                repo,
                format!("eo-editor/{}", session.id),
                string(&main, "sha")?,
            )
        };
        if !valid_repo(&repo) {
            return Err(Error::new("repository", "Invalid head repository."));
        }
        // Compare each touched file with its actual current remote counterpart. Non-overlapping edits merge; overlaps remain local.
        for (path, local) in &mut files {
            let draft = &session.terms[path];
            let base = draft.submitted.as_deref().unwrap_or(&draft.starting);
            let remote = self.content(&repo, path, &parent, token).await?;
            let merged = merge_source(base, local, remote.as_deref(), path)?;
            source::parse(&merged)?;
            *local = merged;
        }
        Ok(Operation {
            repository: repository.clone(),
            base_branch: session.base_branch.clone(),
            id: uuid::Uuid::new_v4().to_string(),
            title: title.trim().to_owned(),
            body,
            original_files,
            files,
            head_repo: repo,
            head_branch: branch,
            parent,
            commit: None,
            account_id: account.id,
        })
    }
    pub async fn create_commit(&self, op: &Operation, token: &str) -> Result<String> {
        let parent = self
            .request(
                Method::GET,
                &format!("/repos/{}/git/commits/{}", op.head_repo, op.parent),
                Some(token),
                None,
            )
            .await?;
        let tree: Vec<_>=op.files.iter().map(|(path,content)|json!({"path":path,"mode":"100644","type":"blob","content":content})).collect();
        let tree = self
            .request(
                Method::POST,
                &format!("/repos/{}/git/trees", op.head_repo),
                Some(token),
                Some(json!({"base_tree":parent["tree"]["sha"],"tree":tree})),
            )
            .await?;
        let commit = self
            .request(
                Method::POST,
                &format!("/repos/{}/git/commits", op.head_repo),
                Some(token),
                Some(json!({"message":op.title,"tree":tree["sha"],"parents":[op.parent]})),
            )
            .await?;
        string(&commit, "sha")
    }
    pub async fn publish(
        &self,
        op: &Operation,
        existing: Option<&Submission>,
        token: &str,
    ) -> Result<Submission> {
        let repository = &op.repository;
        if !valid_repo(repository) || !valid_repo(&op.head_repo) {
            return Err(Error::new("repository", "Invalid submission repository."));
        }
        if let Some(link) = existing {
            crate::store::validate_link(repository, link)?;
            let pr = self.pr(repository, link.number, token).await?;
            if pr["state"] != "open" || pr["user"]["id"] != op.account_id {
                return Err(Error::new(
                    "closed",
                    "The linked PR is closed or belongs to another author.",
                ));
            }
        }
        let commit = op
            .commit
            .as_ref()
            .ok_or_else(|| Error::new("operation", "No commit recorded for this submission."))?;
        let encoded = url::form_urlencoded::byte_serialize(op.head_branch.as_bytes())
            .collect::<String>()
            .replace("%2F", "/");
        let path = format!("/repos/{}/git/refs/heads/{encoded}", op.head_repo);
        match self
            .request(
                Method::GET,
                &path.replace("/git/refs/", "/git/ref/"),
                Some(token),
                None,
            )
            .await
        {
            Ok(reference) if reference["object"]["sha"] == *commit => {}
            Ok(reference) if reference["object"]["sha"] == op.parent => {
                self.request(
                    Method::PATCH,
                    &path,
                    Some(token),
                    Some(json!({"sha":commit,"force":false})),
                )
                .await?;
            }
            Ok(_) => {
                return Err(Error::new(
                    "remote_changed",
                    "The submission branch changed during upload. Your frozen contribution is retained. Export or reconcile it before retrying.",
                ));
            }
            Err(e) if e.code == "not_found" && existing.is_none() => {
                self.request(
                    Method::POST,
                    &format!("/repos/{}/git/refs", op.head_repo),
                    Some(token),
                    Some(json!({"ref":format!("refs/heads/{}",op.head_branch),"sha":commit})),
                )
                .await?;
            }
            Err(e) => return Err(e),
        }
        if let Some(link) = existing {
            crate::store::validate_link(repository, link)?;
            let pr = self.pr(repository, link.number, token).await?;
            if pr["state"] != "open" || pr["user"]["id"] != op.account_id {
                return Err(Error::new(
                    "closed",
                    "The linked PR is closed or belongs to another author.",
                ));
            }
            return submission(repository, &pr, op.account_id);
        }
        let owner = op.head_repo.split('/').next().unwrap_or_default();
        let head = format!("{owner}:{}", op.head_branch);
        let query = url::form_urlencoded::byte_serialize(head.as_bytes()).collect::<String>();
        let found = self
            .request(
                Method::GET,
                &format!("/repos/{repository}/pulls?state=all&head={query}&per_page=100"),
                Some(token),
                None,
            )
            .await?;
        if let Some(pr) = found.as_array().and_then(|a| a.first()) {
            return submission(repository, pr, op.account_id);
        }
        let pr=self.request(Method::POST,&format!("/repos/{repository}/pulls"),Some(token),Some(json!({"title":op.title,"body":op.body,"head":head,"base":op.base_branch,"maintainer_can_modify":true}))).await?;
        submission(repository, &pr, op.account_id)
    }
    pub async fn begin_device(&self, client_id: &str, github_app: bool) -> Result<Value> {
        self.device(
            "https://github.com/login/device/code",
            device_request(client_id, github_app),
        )
        .await
    }
    pub async fn refresh_device(&self, client_id: &str, refresh_token: &str) -> Result<Value> {
        self.device("https://github.com/login/oauth/access_token",json!({"client_id":client_id,"refresh_token":refresh_token,"grant_type":"refresh_token"})).await
    }
    pub async fn poll_device(&self, client_id: &str, device_code: &str) -> Result<Value> {
        self.device("https://github.com/login/oauth/access_token",json!({"client_id":client_id,"device_code":device_code,"grant_type":"urn:ietf:params:oauth:grant-type:device_code"})).await
    }
    async fn device(&self, url: &str, body: Value) -> Result<Value> {
        self.client
            .post(url)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(network)?
            .error_for_status()
            .map_err(network)?
            .json()
            .await
            .map_err(network)
    }
}
pub fn merge_source(base: &str, local: &str, remote: Option<&str>, path: &str) -> Result<String> {
    if remote == Some(base) || (base.is_empty() && remote.is_none()) {
        return Ok(local.to_owned());
    }
    if remote == Some(local) {
        return Ok(local.to_owned());
    }
    let remote=remote.ok_or_else(||Error::new("conflict",format!("{path} was removed remotely. Your draft is retained. Reconcile the term on GitHub before submitting.")))?;
    if base.is_empty() {
        return Err(Error::new(
            "conflict",
            format!(
                "A different term already exists at {path}. Your new term is retained locally."
            ),
        ));
    }
    diffy::merge(base,local,remote).map_err(|_|Error::new("conflict",format!("Overlapping remote changes in {path}. Your draft is retained. Compare the current term on GitHub and reconcile it before submitting.")))
}
pub fn bundled() -> Result<Catalogue> {
    Ok(serde_json::from_str(include_str!(
        "../../../resources/catalogue.json"
    ))?)
}
fn network(error: reqwest::Error) -> Error {
    Error::new(
        "network",
        if error.is_timeout() {
            "GitHub timed out. Your drafts are safe; retry when connected."
        } else {
            "Could not reach GitHub. Your drafts are safe; check your connection and retry."
        },
    )
}
pub fn string(value: &Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::new("github", format!("GitHub response is missing {key}.")))
}
fn verify_pr(repository: &str, pr: &Value, account: &Account) -> Result<()> {
    if pr["state"] != "open"
        || pr["user"]["id"] != account.id
        || pr["base"]["repo"]["full_name"] != repository
    {
        return Err(Error::new(
            "ownership",
            "Choose an open glossary PR authored by your signed-in account.",
        ));
    }
    Ok(())
}
fn submission(repository: &str, pr: &Value, account_id: u64) -> Result<Submission> {
    let number = pr["number"]
        .as_u64()
        .ok_or_else(|| Error::new("github", "Missing PR number."))?;
    let link = Submission {
        number,
        url: format!("https://github.com/{repository}/pull/{number}"),
        head_repo: string(&pr["head"]["repo"], "full_name")?,
        head_branch: string(&pr["head"], "ref")?,
        head_sha: string(&pr["head"], "sha")?,
        account_id,
        state: if pr["merged_at"].is_string() {
            "merged".into()
        } else {
            string(pr, "state")?
        },
    };
    crate::store::validate_link(repository, &link)?;
    Ok(link)
}
/// Durable stage helpers shared by the native command handler and recovery tests.
pub fn record_operation(store: &mut Store, id: &str, operation: &Operation) -> Result<()> {
    store.mutate(id, |s| {
        if let Some(active) = &s.operation
            && active.id != operation.id
        {
            return Err(Error::new(
                "busy",
                "Another submission is already in progress.",
            ));
        }
        if s.repository != operation.repository {
            return Err(Error::new(
                "repository",
                "Submission repository does not match this session.",
            ));
        }
        s.operation = Some(operation.clone());
        Ok(())
    })
}

fn encode_reference(reference: &str) -> String {
    url::form_urlencoded::byte_serialize(reference.as_bytes()).collect()
}
fn same_network(target: &Value, fork: &Value, repository: &str) -> bool {
    if fork["parent"]["full_name"]
        .as_str()
        .is_some_and(|p| p.eq_ignore_ascii_case(repository))
    {
        return true;
    }
    let root = target["source"]["full_name"].as_str().unwrap_or(repository);
    fork["source"]["full_name"]
        .as_str()
        .is_some_and(|p| p.eq_ignore_ascii_case(root))
}

/// GitHub App permissions come from its registration, rather than OAuth scopes.
pub fn device_request(client_id: &str, github_app: bool) -> Value {
    if github_app {
        json!({"client_id":client_id})
    } else {
        json!({"client_id":client_id,"scope":"public_repo"})
    }
}
