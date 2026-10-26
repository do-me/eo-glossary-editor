use clap::{Parser, Subcommand};
use editor_core::{github::bundled, source::Edit, store::Store};
use std::{io::Read, path::PathBuf};
#[derive(Parser)]
#[command(about = "Local EO glossary sessions. Never accesses credentials or publishes.")]
struct Args {
    #[arg(long)]
    database: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    #[cfg(feature = "test-server")]
    Serve {
        #[arg(long)]
        port: u16,
        #[arg(long)]
        test_token: String,
    },
    Sessions,
    New {
        name: String,
    },
    Terms {
        #[arg(long)]
        session: Option<String>,
    },
    Open {
        session: String,
        path: String,
    },
    CreateTerm {
        session: String,
        title: String,
        #[arg(long)]
        team: Vec<String>,
    },
    Edit {
        session: String,
        path: String,
        #[arg(long)]
        revision: u64,
    },
    Export {
        session: String,
    },
    Import {
        file: PathBuf,
    },
    Validate {
        file: PathBuf,
    },
}
fn run() -> editor_core::Result<()> {
    let args = Args::parse();
    let mut store = Store::open(&args.database, &bundled()?)?;
    let value = match args.command {
        #[cfg(feature = "test-server")]
        Command::Serve { port, test_token } => {
            serve(store, port, &test_token)?;
            return Ok(());
        }
        Command::Sessions => serde_json::to_value(store.sessions()?)?,
        Command::New { name } => serde_json::to_value(store.create_session(name)?)?,
        Command::Terms { session } => {
            serde_json::to_value(store.catalogue_terms(session.as_deref())?)?
        }
        Command::Open { session, path } => serde_json::to_value(store.open_term(&session, &path)?)?,
        Command::CreateTerm {
            session,
            title,
            team,
        } => {
            let mut tags = vec!["core".into(), "to be defined".into()];
            tags.extend(team);
            serde_json::to_value(store.new_term(&session, &title, &tags)?)?
        }
        Command::Edit {
            session,
            path,
            revision,
        } => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let edit: Edit = serde_json::from_str(&input)?;
            serde_json::to_value(store.save_edit(&session, &path, revision, edit)?)?
        }
        Command::Export { session } => {
            println!("{}", store.export(&session)?);
            return Ok(());
        }
        Command::Import { file } => {
            serde_json::to_value(store.import(&std::fs::read_to_string(file)?)?)?
        }
        Command::Validate { file } => {
            serde_json::to_value(editor_core::source::parse(&std::fs::read_to_string(file)?)?)?
        }
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{}: {}", e.code, e.message);
        std::process::exit(1);
    }
}

#[cfg(feature = "test-server")]
fn serve(mut store: Store, port: u16, token: &str) -> editor_core::Result<()> {
    let server = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|_| editor_core::Error::new("server", "Cannot bind test server"))?;
    for mut req in server.incoming_requests() {
        let origin = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Origin"))
            .map(|h| h.value.as_str());
        if origin != Some("http://127.0.0.1:1420") {
            let _ = req.respond(tiny_http::Response::empty(403));
            continue;
        }
        let headers = [
            ("Access-Control-Allow-Origin", "http://127.0.0.1:1420"),
            ("Access-Control-Allow-Headers", "content-type,x-test-token"),
            ("Access-Control-Allow-Methods", "POST, OPTIONS"),
        ];
        let response = if req.method() == &tiny_http::Method::Options {
            tiny_http::Response::from_string("")
        } else if req.method() != &tiny_http::Method::Post
            || req
                .headers()
                .iter()
                .find(|h| h.field.equiv("X-Test-Token"))
                .map(|h| h.value.as_str())
                != Some(token)
        {
            tiny_http::Response::from_string("Forbidden").with_status_code(403)
        } else {
            let mut text = String::new();
            req.as_reader()
                .take(1024 * 1024)
                .read_to_string(&mut text)?;
            let result = (|| {
                let v: serde_json::Value = serde_json::from_str(&text)?;
                let name = v["name"].as_str().unwrap_or("");
                if name == "account" {
                    return Ok(serde_json::json!({"account":null,"clientIdConfigured":false}));
                }
                if name == "check_updates" {
                    let id = v["data"]["session"].as_str().unwrap_or("");
                    let session = store.session(id)?;
                    let catalogue = store.latest_catalogue(&session.repository)?;
                    return Ok(
                        serde_json::json!({"repository":catalogue.repository,"sha":catalogue.sha,"branch":catalogue.branch,"checkedAt":editor_core::model::now()}),
                    );
                }
                if name == "fixture_main_update" {
                    let repository = v["data"]["repository"]
                        .as_str()
                        .unwrap_or(editor_core::model::UPSTREAM);
                    let mut catalogue = store.latest_catalogue(repository)?;
                    let path = v["data"]["path"]
                        .as_str()
                        .unwrap_or("docs/terms/accuracy.md");
                    let original = catalogue
                        .files
                        .get(path)
                        .ok_or_else(|| editor_core::Error::new("fixture", "Term not found."))?;
                    let source = editor_core::source::apply_edit(
                        original,
                        serde_json::from_value(v["data"]["edit"].clone())?,
                    )?;
                    catalogue.files.insert(path.into(), source);
                    catalogue.sha = format!(
                        "{:040x}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("clock")
                            .as_nanos()
                    );
                    catalogue.fetched_at = editor_core::model::now();
                    store.cache_remote(&catalogue)?;
                    return Ok(serde_json::json!({"sha":catalogue.sha}));
                }
                if name == "configure_repository" {
                    // Offline fixture transport for UI tests. The native app always downloads GitHub data.
                    let repository =
                        editor_core::repository::parse(v["data"]["url"].as_str().unwrap_or(""))?;
                    if repository != "do-me/eo-glossary"
                        && repository != editor_core::model::UPSTREAM
                    {
                        return Err(editor_core::Error::new(
                            "fixture",
                            "Repository not present in the test fixture.",
                        ));
                    }
                    let mut catalogue = editor_core::github::bundled()?;
                    catalogue.repository = repository;
                    let session = store.switch_repository(&catalogue)?;
                    return Ok(
                        serde_json::json!({"session":session,"bootstrap":store.bootstrap()?}),
                    );
                }
                editor_core::commands::dispatch(&mut store, name, &v["data"])
            })();
            match result {
                Ok(v) => tiny_http::Response::from_string(v.to_string()),
                Err(e) => tiny_http::Response::from_string(serde_json::to_string(&e)?)
                    .with_status_code(400),
            }
        };
        let mut response = response;
        for (k, v) in headers {
            response.add_header(tiny_http::Header::from_bytes(k, v).expect("test header"));
        }
        response.add_header(
            tiny_http::Header::from_bytes("Content-Type", "application/json").expect("test header"),
        );
        let _ = req.respond(response);
    }
    Ok(())
}
