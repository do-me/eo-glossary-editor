//! Canonical GitHub identities. URLs never become API hosts or filesystem paths.
use crate::{Error, Result};
pub fn parse(input: &str) -> Result<String> {
    let input = input.trim();
    let path = if input.starts_with("https://") {
        let url = url::Url::parse(input).map_err(|_| invalid())?;
        if url.host_str() != Some("github.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid());
        }
        url.path().trim_matches('/').to_owned()
    } else {
        input.trim_end_matches('/').to_owned()
    };
    let repository = path
        .strip_suffix(".git")
        .unwrap_or(&path)
        .to_ascii_lowercase();
    if !valid(&repository) {
        return Err(invalid());
    }
    Ok(repository)
}
pub fn valid(repository: &str) -> bool {
    let parts: Vec<_> = repository.split('/').collect();
    parts.len() == 2
        && parts.iter().enumerate().all(|(i, p)| {
            !p.is_empty()
                && p.len() <= if i == 0 { 39 } else { 100 }
                && !p.starts_with('.')
                && !p.ends_with('.')
                && *p != ".."
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}
pub fn default_repository() -> String {
    crate::model::UPSTREAM.into()
}
pub fn default_branch() -> String {
    "main".into()
}
fn invalid() -> Error {
    Error::new(
        "repository",
        "Enter a GitHub repository URL such as https://github.com/do-me/eo-glossary, or owner/repository. Other hosts and file/subdirectory URLs are not supported.",
    )
}
