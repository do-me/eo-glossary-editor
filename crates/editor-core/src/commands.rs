//! Credential-free local command boundary, also used by the agent/testing interface.
use crate::{Error, Result, source::Edit, store::Store};
use serde_json::{Value, json};
fn payload<T: serde::de::DeserializeOwned>(v: &Value, k: &str) -> Result<T> {
    Ok(serde_json::from_value(v[k].clone())?)
}
pub fn dispatch(store: &mut Store, name: &str, data: &Value) -> Result<Value> {
    let id = || payload::<String>(data, "session");
    match name {
        "bootstrap" => store.bootstrap(),
        "preferences" => Ok(serde_json::to_value(store.preferences()?)?),
        "save_preferences" => Ok(serde_json::to_value(
            store.save_preferences(payload(data, "preferences")?)?,
        )?),
        "apply_updates" => Ok(serde_json::to_value(
            store.apply_updates(&id()?, &payload::<String>(data, "sha")?)?,
        )?),
        "sync_report" => Ok(serde_json::to_value(store.sync_report(&id()?)?)?),
        "resolve_update" => Ok(serde_json::to_value(store.resolve_update(
            &id()?,
            &payload::<String>(data, "path")?,
            payload(data, "revision")?,
            &payload::<String>(data, "sha")?,
            payload(data, "choice")?,
        )?)?),
        "sessions" => Ok(serde_json::to_value(store.sessions()?)?),
        "session" => Ok(serde_json::to_value(store.session(&id()?)?)?),
        "new_session" => Ok(serde_json::to_value(
            store.create_session(payload(data, "name")?)?,
        )?),
        "follow_up" => Ok(serde_json::to_value(store.follow_up(&id()?)?)?),
        "rename_session" => {
            store.rename(&id()?, &payload::<String>(data, "name")?)?;
            Ok(Value::Null)
        }
        "terms" => Ok(serde_json::to_value(store.catalogue_terms(Some(&id()?))?)?),
        "open" => Ok(serde_json::to_value(
            store.open_term(&id()?, &payload::<String>(data, "path")?)?,
        )?),
        "edit" => Ok(serde_json::to_value(store.save_edit(
            &id()?,
            &payload::<String>(data, "path")?,
            payload(data, "revision")?,
            payload::<Edit>(data, "edit")?,
        )?)?),
        "new_term" => Ok(serde_json::to_value(store.new_term(
            &id()?,
            &payload::<String>(data, "title")?,
            &payload::<Vec<String>>(data, "tags")?,
        )?)?),
        "workspace" => {
            store.save_workspace(&id()?, payload(data, "workspace")?)?;
            Ok(Value::Null)
        }
        "changes" => Ok(serde_json::to_value(
            store.changes(&id()?, data["all"].as_bool().unwrap_or(false))?,
        )?),
        "revert" => {
            store.revert(
                &id()?,
                &payload::<String>(data, "path")?,
                payload(data, "revision")?,
            )?;
            Ok(Value::Null)
        }
        "export" => Ok(json!(store.export(&id()?)?)),
        "import" => Ok(serde_json::to_value(
            store.import(&payload::<String>(data, "content")?)?,
        )?),
        _ => Err(Error::new("command", "Unknown application command.")),
    }
}
