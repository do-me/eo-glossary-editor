# Project context

Read `PLAN.md` and the relevant documents under `docs/` before implementation. Read `docs/VALIDATION.md` for actual implementation coverage. Application code and local verification are present.

## Product invariants

- Tauri native application with Rust backend and Svelte/TypeScript/Tailwind frontend.
- No hosted application or authentication service.
- Search, session management, and term editing are the main entry points.
- Multiple term tabs, automatic local saving, and restart recovery are required.
- One session spans multiple term files and links to at most one GitHub PR. Updates reuse that PR.
- Closing a tab preserves its saved draft. Saving locally never publishes remotely.
- New terms display a complete skeleton immediately; required heading lines remain outside editable content and are protected by Rust.
- Preserve untouched Markdown, unknown frontmatter, alternative definitions, and existing tags.
- Session progress and GitHub submission state are separate from a term’s scientific discussion/approval status.

## Implementation boundaries

- Rust owns persistence, source integrity, credentials, and GitHub mutations.
- Follow the GitHub integration decision record; do not introduce a sidecar or hosted dependency without a concrete reason.
- Keep CSS minimal, using shared tokens and Tailwind utilities. Keep the document editor visually central.
- Prefer a narrow, tested vertical slice over speculative generalized infrastructure.
- Use meaningful corpus, crash-recovery, concurrency, and end-to-end tests as described in `docs/DELIVERY.md`.
- Exercise remote integration tests in a dedicated sandbox repository. Do not publish test edits to the production glossary.
- Keep the human’s existing glossary checkout and other repositories untouched.
