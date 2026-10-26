#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use editor_core::{
    Error, Result,
    github::{self, Github},
    model::*,
    store::Store,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

type Database = Arc<Mutex<Store>>;
struct State {
    database: Database,
    github: Github,
    credentials: Mutex<Option<Credential>>,
    device: Mutex<Option<Device>>,
    submission: tokio::sync::Mutex<()>,
    persisted: AtomicBool,
    exit_authorized: AtomicBool,
    auth_generation: AtomicU64,
    refresh: tokio::sync::Mutex<()>,
}
#[derive(Serialize, Deserialize)]
struct Credential {
    token: String,
    account: Account,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_at: Option<u64>,
}
impl Drop for Credential {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
        if let Some(token) = &mut self.refresh_token {
            token.zeroize();
        }
    }
}
struct Device {
    client_id: String,
    code: Zeroizing<String>,
    expires: Instant,
    next: Instant,
    interval: Duration,
    remember: bool,
}
fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| {
        Error::new(
            "state",
            "The local state lock failed. Restart the app; saved drafts are retained.",
        )
    })
}
async fn database<T: Send + 'static>(
    db: &Database,
    f: impl FnOnce(&mut Store) -> Result<T> + Send + 'static,
) -> Result<T> {
    let db = db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = lock(&db)?;
        f(&mut store)
    })
    .await
    .map_err(|_| Error::new("storage", "The storage worker stopped."))?
}
fn payload<T: serde::de::DeserializeOwned>(value: &Value, key: &str) -> Result<T> {
    serde_json::from_value(value[key].clone()).map_err(Error::from)
}
fn keyring() -> Result<keyring::Entry> {
    keyring::Entry::new("org.ceos.glossary-editor", "github").map_err(|_| {
        Error::new(
            "keyring",
            "Secure credential storage is unavailable. Sign in for this launch only.",
        )
    })
}
fn auth_configuration(store: &Store) -> Result<(Option<String>, String)> {
    let saved = store.preference("client_id")?;
    let client = saved
        .clone()
        .or_else(|| option_env!("EO_GITHUB_CLIENT_ID").map(str::to_owned));
    let kind = store.preference("auth_kind")?.unwrap_or_else(|| {
        if saved.is_some() {
            "oauth_app".into()
        } else {
            option_env!("EO_GITHUB_AUTH_KIND")
                .unwrap_or("github_app")
                .into()
        }
    });
    Ok((client, kind))
}
fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
async fn token(state: &State) -> Result<Zeroizing<String>> {
    {
        let credentials = lock(&state.credentials)?;
        let credential = credentials.as_ref().ok_or_else(|| {
            Error::new(
                "authentication",
                "Authenticate with GitHub before submitting.",
            )
        })?;
        if credential.expires_at.is_none_or(|t| t > unix_time() + 60) {
            return Ok(Zeroizing::new(credential.token.clone()));
        }
    }
    let _guard = state.refresh.lock().await;
    let generation = state.auth_generation.load(Ordering::SeqCst);
    let (client, refresh, account) = {
        let credentials = lock(&state.credentials)?;
        let credential = credentials
            .as_ref()
            .ok_or_else(|| Error::new("authentication", "Sign in to GitHub again."))?;
        if credential.expires_at.is_none_or(|t| t > unix_time() + 60) {
            return Ok(Zeroizing::new(credential.token.clone()));
        }
        (
            credential.client_id.clone().ok_or_else(|| {
                Error::new(
                    "authentication",
                    "Your GitHub session expired. Sign in again.",
                )
            })?,
            Zeroizing::new(credential.refresh_token.clone().ok_or_else(|| {
                Error::new(
                    "authentication",
                    "Your GitHub session expired. Sign in again.",
                )
            })?),
            credential.account.clone(),
        )
    };
    let response = state.github.refresh_device(&client, &refresh).await?;
    let access = github::string(&response, "access_token").map_err(|_| {
        Error::new(
            "authentication",
            "GitHub authorization expired or was revoked. Sign in again.",
        )
    })?;
    if generation != state.auth_generation.load(Ordering::SeqCst) {
        return Err(Error::new(
            "cancelled",
            "Authentication changed during token refresh.",
        ));
    }
    let credential = Credential {
        token: access,
        account,
        client_id: Some(client),
        refresh_token: response["refresh_token"].as_str().map(str::to_owned),
        expires_at: response["expires_in"]
            .as_u64()
            .map(|seconds| unix_time() + seconds),
    };
    let value = Zeroizing::new(credential.token.clone());
    if state.persisted.load(Ordering::SeqCst) {
        let encoded = Zeroizing::new(serde_json::to_string(&credential)?);
        let stored = keyring().and_then(|entry| {
            entry
                .set_password(&encoded)
                .map_err(|_| Error::new("keyring", "Could not remember the renewed sign-in."))
        });
        if stored.is_err() {
            *lock(&state.credentials)? = Some(credential);
            state.persisted.store(false, Ordering::SeqCst);
            return Err(Error::new(
                "keyring",
                "The renewed sign-in is available for this launch, but could not be remembered in secure storage. Unlock the credential store and sign in again to remember it.",
            ));
        }
    }
    *lock(&state.credentials)? = Some(credential);
    Ok(value)
}
async fn sign_in(
    state: &State,
    token: String,
    remember: bool,
    device: Option<(&str, &Value)>,
) -> Result<Value> {
    let generation = state.auth_generation.load(Ordering::SeqCst);
    let secret = Zeroizing::new(token);
    let account = state.github.account(&secret).await?;
    if generation != state.auth_generation.load(Ordering::SeqCst) {
        return Err(Error::new("cancelled", "Authentication was cancelled."));
    }
    let credential = Credential {
        token: secret.to_string(),
        account: account.clone(),
        client_id: device.map(|(id, _)| id.to_owned()),
        refresh_token: device.and_then(|(_, v)| v["refresh_token"].as_str().map(str::to_owned)),
        expires_at: device.and_then(|(_, v)| {
            v["expires_in"]
                .as_u64()
                .map(|seconds| unix_time() + seconds)
        }),
    };
    if remember {
        keyring()?.set_password(&Zeroizing::new(serde_json::to_string(&credential)?)).map_err(|_|Error::new("keyring","Could not save to the system credential store. Retry with Remember me unchecked."))?;
    } else if state.persisted.load(Ordering::SeqCst) {
        match keyring()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(_) => {
                return Err(Error::new(
                    "keyring",
                    "Could not remove an earlier saved credential from the system store.",
                ));
            }
        }
    }
    state.persisted.store(remember, Ordering::SeqCst);
    *lock(&state.credentials)? = Some(credential);
    Ok(json!(account))
}
async fn latest_snapshot(state: &State, repository: String) -> Result<Catalogue> {
    let secret = token(state).await.ok();
    let auth = secret.as_deref().map(|s| s.as_str());
    let (repository, branch, sha) = state.github.repository_head(&repository, auth).await?;
    let repo = repository.clone();
    let revision = sha.clone();
    let cached = database(&state.database, move |s| {
        Ok(s.catalogue_for(&repo, &revision).ok())
    })
    .await?;
    let mut catalogue = match cached {
        Some(c) => c,
        None => state.github.snapshot_sha(&repository, &sha, auth).await?,
    };
    catalogue.branch = branch;
    catalogue.fetched_at = now();
    Ok(catalogue)
}
#[tauri::command]
async fn request(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    name: String,
    data: Value,
) -> Result<Value> {
    match name.as_str() {
        "quit" => {
            state.exit_authorized.store(true, Ordering::SeqCst);
            app.exit(0);
            Ok(Value::Null)
        }
        "account" => {
            let (client, kind) = database(&state.database, |s| auth_configuration(s)).await?;
            let configured = client.is_some();
            let account = lock(&state.credentials)?
                .as_ref()
                .map(|c| c.account.clone());
            Ok(json!({"account":account,"clientIdConfigured":configured,"kind":kind}))
        }
        "configure_auth" => {
            let kind = data["kind"].as_str().unwrap_or("github_app").to_owned();
            if !matches!(kind.as_str(), "github_app" | "oauth_app") {
                return Err(Error::new("auth_kind", "Choose GitHub App or OAuth app."));
            }
            let id: String = payload(&data, "clientId")?;
            if id.is_empty() || id.len() > 100 || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
                return Err(Error::new(
                    "client_id",
                    "Enter the public client ID of your registered GitHub App or OAuth app.",
                ));
            }
            database(&state.database, move |s| {
                s.set_preference("client_id", &id)?;
                s.set_preference("auth_kind", &kind)?;
                Ok(Value::Null)
            })
            .await
        }
        "sign_in_token" => {
            sign_in(
                &state,
                payload(&data, "token")?,
                payload(&data, "remember")?,
                None,
            )
            .await
        }
        "sign_out" => {
            state.auth_generation.fetch_add(1, Ordering::SeqCst);
            if state.persisted.load(Ordering::SeqCst) {
                match keyring()?.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(_) => {
                        return Err(Error::new(
                            "keyring",
                            "Could not remove your saved credential. Unlock the system credential store and retry.",
                        ));
                    }
                };
            }
            state.persisted.store(false, Ordering::SeqCst);
            *lock(&state.credentials)? = None;
            *lock(&state.device)? = None;
            Ok(Value::Null)
        }
        "auth_begin" => {
            state.auth_generation.fetch_add(1, Ordering::SeqCst);
            let (id, kind) = database(&state.database, |s| auth_configuration(s)).await?;
            let id=id.ok_or_else(||Error::new("client_id","Register the editor's GitHub App and configure its public client ID, or use a personal access token."))?;
            let is_github_app = kind == "github_app";
            let v = state.github.begin_device(&id, is_github_app).await?;
            if let Some(e) = v["error_description"].as_str() {
                return Err(Error::new("authentication", e));
            }
            let code = github::string(&v, "device_code")?;
            let url = github::string(&v, "verification_uri")?;
            if url != "https://github.com/login/device" {
                return Err(Error::new(
                    "authentication",
                    "GitHub returned an unexpected authentication URL.",
                ));
            }
            let interval = Duration::from_secs(v["interval"].as_u64().unwrap_or(5).max(5));
            *lock(&state.device)? = Some(Device {
                client_id: id,
                code: Zeroizing::new(code),
                expires: Instant::now()
                    + Duration::from_secs(v["expires_in"].as_u64().unwrap_or(900)),
                next: Instant::now() + interval,
                interval,
                remember: payload(&data, "remember")?,
            });
            Ok(
                json!({"userCode":v["user_code"],"verificationUrl":url,"interval":interval.as_secs()}),
            )
        }
        "auth_cancel" => {
            state.auth_generation.fetch_add(1, Ordering::SeqCst);
            *lock(&state.device)? = None;
            Ok(Value::Null)
        }
        "auth_poll" => {
            let generation = state.auth_generation.load(Ordering::SeqCst);
            let mut device = lock(&state.device)?.take().ok_or_else(|| {
                Error::new("authentication", "Start GitHub authentication again.")
            })?;
            if Instant::now() > device.expires {
                return Err(Error::new(
                    "authentication",
                    "The code expired. Start again.",
                ));
            }
            if Instant::now() < device.next {
                *lock(&state.device)? = Some(device);
                return Ok(json!({"pending":true}));
            }
            let v = match state
                .github
                .poll_device(&device.client_id, &device.code)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    *lock(&state.device)? = Some(device);
                    return Err(e);
                }
            };
            if generation != state.auth_generation.load(Ordering::SeqCst) {
                return Err(Error::new("cancelled", "Authentication was cancelled."));
            }
            if let Some(t) = v["access_token"].as_str() {
                let account = sign_in(
                    &state,
                    t.to_owned(),
                    device.remember,
                    Some((&device.client_id, &v)),
                )
                .await?;
                return Ok(json!({"account":account}));
            }
            match v["error"].as_str() {
                Some("authorization_pending") => {}
                Some("slow_down") => device.interval += Duration::from_secs(5),
                _ => {
                    return Err(Error::new(
                        "authentication",
                        v["error_description"]
                            .as_str()
                            .unwrap_or("GitHub authorization was declined or expired."),
                    ));
                }
            };
            device.next = Instant::now() + device.interval;
            *lock(&state.device)? = Some(device);
            Ok(json!({"pending":true}))
        }
        "save_preferences" => {
            let preferences: Preferences = payload(&data, "preferences")?;
            if !(75..=200).contains(&preferences.zoom) {
                return Err(Error::new("zoom", "Zoom must be between 75% and 200%."));
            }
            if let Some(window) = app.get_webview_window("main") {
                window
                    .set_zoom(f64::from(preferences.zoom) / 100.0)
                    .map_err(|_| Error::new("zoom", "Could not change the window zoom."))?;
            }
            database(&state.database, move |s| {
                Ok(serde_json::to_value(s.save_preferences(preferences)?)?)
            })
            .await
        }
        "check_updates" => {
            let id: String = payload(&data, "session")?;
            let read = id.clone();
            let session = database(&state.database, move |s| s.session(&read)).await?;
            let catalogue = latest_snapshot(&state, session.repository.clone()).await?;
            let secret = token(&state).await.ok();
            let auth = secret.as_deref().map(|s| s.as_str());
            let pr_state = if let Some(link) = &session.submission {
                state.github.public_pr(&session.repository,link.number,auth).await.ok().map(|v|json!({"number":link.number,"state":if v["merged_at"].is_string(){"merged"}else{v["state"].as_str().unwrap_or("open")}}))
            } else {
                None
            };
            let response = json!({"repository":catalogue.repository,"sha":catalogue.sha,"branch":catalogue.branch,"checkedAt":now()});
            database(&state.database, move |s| {
                s.cache_remote(&catalogue)?;
                if let Some(status) = pr_state {
                    s.set_preference(&format!("checked_pr:{id}"), &status.to_string())?;
                }
                Ok(())
            })
            .await?;
            let selected = database(&state.database, |s| Ok(s.catalogue(None)?.repository)).await?;
            if selected != session.repository
                && let Ok(catalogue) = latest_snapshot(&state, selected).await
            {
                database(&state.database, move |s| s.cache_remote(&catalogue)).await?;
            }
            Ok(response)
        }
        "configure_repository" => {
            let _guard = state.submission.try_lock().map_err(|_| {
                Error::new(
                    "busy",
                    "Wait for the current submission before switching repositories.",
                )
            })?;
            let repository = editor_core::repository::parse(&payload::<String>(&data, "url")?)?;
            let t = token(&state).await.ok();
            let catalogue = state
                .github
                .download_repository(&repository, t.as_deref().map(|s| s.as_str()))
                .await?;
            database(&state.database, move |s| {
                let session = s.switch_repository(&catalogue)?;
                Ok(json!({"session":session,"bootstrap":s.bootstrap()?}))
            })
            .await
        }
        "refresh" => {
            let repository =
                database(&state.database, |s| Ok(s.catalogue(None)?.repository)).await?;
            let t = token(&state).await.ok();
            let catalogue = state
                .github
                .download_repository(&repository, t.as_deref().map(|s| s.as_str()))
                .await?;
            database(&state.database, move |s| {
                s.install_catalogue(&catalogue)?;
                s.bootstrap()
            })
            .await
        }
        "authored_prs" => {
            let t = token(&state).await?;
            let account = state.github.account(&t).await?;
            let repository =
                database(&state.database, |s| Ok(s.catalogue(None)?.repository)).await?;
            state.github.authored_prs(&repository, &account, &t).await
        }
        "import_pr" => {
            let t = token(&state).await?;
            let number = payload(&data, "number")?;
            let account = state.github.account(&t).await?;
            let repository =
                database(&state.database, |s| Ok(s.catalogue(None)?.repository)).await?;
            let (base, session) = state
                .github
                .import_pr(&repository, number, &account, &t)
                .await?;
            database(&state.database, move |s| {
                Ok(serde_json::to_value(s.import_pr(base, session)?)?)
            })
            .await
        }
        "submit" => {
            let _guard = state.submission.try_lock().map_err(|_| {
                Error::new(
                    "busy",
                    "A submission is already running. Your drafts can still be edited.",
                )
            })?;
            let t = token(&state).await?;
            let account = state.github.account(&t).await?;
            let id: String = payload(&data, "session")?;
            let read_id = id.clone();
            let session = database(&state.database, move |s| s.session(&read_id)).await?;
            let mut operation = state
                .github
                .prepare(
                    &session,
                    &account,
                    &t,
                    payload(&data, "title")?,
                    payload(&data, "body")?,
                )
                .await?;
            let record_id = id.clone();
            let op = operation.clone();
            database(&state.database, move |s| {
                github::record_operation(s, &record_id, &op)
            })
            .await?;
            if operation.commit.is_none() {
                operation.commit = Some(state.github.create_commit(&operation, &t).await?);
                let record_id = id.clone();
                let op = operation.clone();
                database(&state.database, move |s| {
                    github::record_operation(s, &record_id, &op)
                })
                .await?;
            }
            let link = state
                .github
                .publish(&operation, session.submission.as_ref(), &t)
                .await?;
            let response = serde_json::to_value(&link)?;
            database(&state.database, move |s| {
                s.finish_submission(&id, &operation, link)
            })
            .await?;
            Ok(response)
        }
        "export_file" => {
            let id: String = payload(&data, "session")?;
            let content = database(&state.database, move |s| s.export(&id)).await?;
            tauri::async_runtime::spawn_blocking(move || {
                let path = app
                    .dialog()
                    .file()
                    .add_filter("Session", &["json"])
                    .set_file_name("glossary-session.json")
                    .blocking_save_file();
                if let Some(path) = path {
                    let path = path
                        .into_path()
                        .map_err(|_| Error::new("path", "Choose a local file."))?;
                    std::fs::write(path, content)?;
                }
                Ok(Value::Null)
            })
            .await
            .map_err(|_| Error::new("io", "Export dialog stopped."))?
        }
        "import_file" => {
            let content = tauri::async_runtime::spawn_blocking(move || {
                app.dialog()
                    .file()
                    .add_filter("Session", &["json"])
                    .blocking_pick_file()
                    .map(|p| {
                        let path = p
                            .into_path()
                            .map_err(|_| Error::new("path", "Choose a local file."))?;
                        if std::fs::metadata(&path)?.len() > 25 * 1024 * 1024 {
                            return Err(Error::new(
                                "size",
                                "Session files must be smaller than 25 MiB.",
                            ));
                        }
                        Ok(std::fs::read_to_string(path)?)
                    })
                    .transpose()
            })
            .await
            .map_err(|_| Error::new("io", "Import dialog stopped."))??;
            database(&state.database, move |s| match content {
                Some(c) => Ok(serde_json::to_value(s.import(&c)?)?),
                None => Ok(Value::Null),
            })
            .await
        }
        _ => {
            database(&state.database, move |store| {
                editor_core::commands::dispatch(store, &name, &data)
            })
            .await
        }
    }
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("sessions.sqlite3");
            let store = Store::open(&path, &github::bundled()?)?;
            if let Some(window) = app.get_webview_window("main") {
                window.set_zoom(f64::from(store.preferences()?.zoom) / 100.0)?;
            }
            let credentials = keyring()
                .ok()
                .and_then(|e| e.get_password().ok())
                .and_then(|s| serde_json::from_str::<Credential>(&Zeroizing::new(s)).ok());
            let persisted = credentials.is_some();
            app.manage(State {
                database: Arc::new(Mutex::new(store)),
                github: Github::default(),
                credentials: Mutex::new(credentials),
                device: Mutex::new(None),
                submission: tokio::sync::Mutex::new(()),
                persisted: AtomicBool::new(persisted),
                exit_authorized: AtomicBool::new(false),
                auth_generation: AtomicU64::new(0),
                refresh: tokio::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request])
        .build(tauri::generate_context!())
        .expect("desktop runtime")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event
                && !app.state::<State>().exit_authorized.load(Ordering::SeqCst)
            {
                api.prevent_exit();
                let _ = app.emit("quit-requested", ());
            }
        });
}
