# Decision 003 — personal access tokens are primary

Confirmed by the maintainer on 27 October 2026. This supersedes the authentication priority in Decisions 001 and 002.

Use a user-provided GitHub personal access token as the normal route. Show the token field directly in Settings, link to GitHub token creation, and explain the public_repo scope used for the public glossary. Remembered tokens stay in the native credential store; temporary sign-in remains available. Clear the input on transfer to Rust and exclude credentials from workspace/session exports.

GitHub App/OAuth device-flow support is retained as an optional, collapsed experiment. It may be revisited in the future, but there is no commitment to deploy it and no registration/client ID prerequisite for release or ordinary users. Direct Rust integration remains; no hosted service or CLI sidecar is introduced.

Classic public_repo tokens are the currently tested contributor route. Fine-grained tokens remain usable where their repository/owner permissions support the workflow; cross-owner fork and organization restrictions must be respected. See [GitHub’s personal-token documentation](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens).
