# Decision 001 — direct GitHub integration or a `gh` sidecar

Status: implemented direct client; authentication priority superseded by [Decision 003](003-personal-access-tokens.md); Decision 002 retains the historical app comparison
Date: 26 October 2026

## Context

The user prefers a custom Tauri application with a Rust backend and asks whether GitHub CLI should be bundled as a sidecar. The application needs browser-assisted GitHub authentication, remote glossary snapshots, personal forks where necessary, and one resumable PR per editing session. The user cannot operate a hosted authentication/backend service.

The application must work for non-technical experts on a clean machine. Installing Git or GitHub CLI separately must not become an onboarding step.

## Decision

Use a **direct Rust GitHub client and native device authorization** as the initial implementation. Do not bundle `gh` by default. Use a project-owned GitHub App with device flow enabled, keeping user credentials in native secure storage. OAuth-app compatibility remains available; see Decision 002.

Keep the GitHub operations behind a small domain interface so the choice can be revisited if an implementation experiment reveals a substantial problem. Do not build two complete backends in parallel.

## Comparison

| Concern | Direct Rust device flow/API | Bundled `gh` |
|---|---|---|
| Hosted authentication service | Not required | Not required for CLI browser authentication |
| Initial identity setup | Maintainer registers the project OAuth app once | Uses CLI authentication behaviour; packaging integration still needed |
| User installation | One editor installer | One editor installer if every required sidecar is bundled |
| Native interface | Typed progress, errors, cancellation, and retry control | Subprocess lifecycle and interactive authentication output need adaptation |
| Permissions | Project can request the needed public-repository scope | Default CLI authentication requests broader standard capabilities |
| Credentials | Application-owned vault namespace and account binding | CLI credential/config behaviour must be isolated and tested |
| Atomic multi-file submission | Git tree/commit/reference APIs from Rust | `gh api` can invoke the same APIs; orchestration is still custom |
| Local Git dependency | None for this design | `gh` is not Git; commands that invoke Git require it separately |
| Packaging | Native application and normal libraries | OS/architecture binaries, notices, updates, integrity checks, and signing |
| Testing | Mock transport and deterministic operation journal | Mocked process output plus real-binary compatibility tests |
| Long-term responsibility | Maintain a small explicit API surface | Maintain CLI version compatibility and process handling as well as the session workflow |

The CLI is useful, but it does not provide the product’s central abstractions: local scientific drafts, source-preserving editing, session identity, frozen revisions, or retry-safe one-PR coordination. Those still belong in Rust application code.

## Why this default fits the interface

“Authenticate with GitHub” should show a code, open the external browser, and expose clear progress without a terminal window. The native device-flow implementation can report these states directly.

GitHub documents a device flow that exchanges a public client ID and device code for a user token without a client secret. This enables the zero-hosted-backend design. Registering the OAuth app is a GitHub configuration task; it is not running a service.

The GitHub CLI documents browser-based login and native credential storage, but also a plaintext fallback when secure storage is unavailable. The editor’s desired behaviour is different: fail visibly or offer explicit temporary sign-in rather than silently persisting a token in plaintext. Adapting CLI behaviour safely would need deliberate handling.

## Conditions for reconsidering a sidecar

Revisit this decision only if an experiment establishes that:

- GitHub authentication or organization policy is materially easier for target experts through the CLI;
- an essential API operation is too costly or unreliable to own directly;
- the maintainer deliberately prefers relying on the GitHub CLI’s authentication identity and accepts its additional distribution/permission tradeoffs.

A developer using installed `gh` to exercise APIs or inspect test PRs does not make it an application runtime dependency.

## If `gh` is selected later

Use Tauri’s external-binary support and pin a specific CLI release. Supply binaries for each supported target triple, verify download integrity in the release pipeline, and include required redistribution notices.

Launch the sidecar only from Rust with fixed argument arrays, a bounded environment, cancellation, output limits, and timeouts. Do not invoke a shell or expose a generic command runner to the frontend. Do not log authorization output or token-bearing environment values.

Prefer `gh api` with structured JSON for remote operations where that avoids implicit working-directory and Git dependencies. Explicitly identify which commands require Git; bundling `gh` alone must not accidentally make those commands part of the runtime path.

Define whether CLI configuration is application-owned or shared before shipping. Do not automatically consume or alter the contributor’s existing CLI account, credentials, Git configuration, SSH keys, or default repository. Confirm platform secure-storage behaviour in the packaged app, not only in a terminal.

Even with a sidecar, keep durable submission intents, conflict detection, source checks, and PR deduplication in the application core. The CLI does not substitute for those guarantees.

## References

- [GitHub device authorization](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow).
- [GitHub CLI login behaviour and credential-storage fallback](https://cli.github.com/manual/gh_auth_login).
- [GitHub CLI API command](https://cli.github.com/manual/gh_api).
- [Tauri sidecar binaries and architecture-specific packaging](https://v2.tauri.app/develop/sidecar/).
