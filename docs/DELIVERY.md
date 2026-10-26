# Delivery plan and acceptance gates

## 1. Implementation approach

Build a small number of complete user journeys. Establish source integrity, local durability, and submission identity early; these are harder to retrofit than visual styling.

Do not start by adding every editor feature or by building a generic Git client. Each milestone must make the expert workflow more complete and produce something directly reviewable.

The application is now implemented locally. The phases below remain the original acceptance plan; `VALIDATION.md` records which checks were actually run and which release gates remain.

## 2. Milestones

### M0 — prove the risky integrations

Deliver:

- A fixture corpus drawn from the current glossary, with provenance and a fixed source commit.
- A Milkdown/Svelte integration experiment against representative Markdown, including alternative definitions, empty headings, tables, Unicode, and frontmatter separation.
- A report comparing untouched round trips, paragraph edits, metadata-only edits, and unsupported blocks. Do not accept broad reformatting of unrelated content.
- A native GitHub device-flow experiment with a project client ID, cancellation, expiry, account verification, and secure credential persistence.
- A sandbox-repository experiment creating a multi-file commit and PR through Git data APIs, including a fork owned by an account without upstream write access.

Exit gate: the selected editor and GitHub path can support the core product without hosted services or source loss. If the editor fails, test the fallback before implementing the complete UI. Resolve the default sidecar decision using this evidence.

### M1 — application shell, catalogue, and sessions

Deliver:

- Tauri 2 + Rust + Svelte 5 + TypeScript + Vite + Tailwind 4 scaffold and locked dependencies.
- The minimal navigation, search/results, Sessions list, and Settings/account surface.
- Immutable source snapshots and a cached, locally searchable term catalogue.
- SQLite migrations, session creation/switching, workspace persistence, and diagnostic errors.
- A browser-test transport adapter for UI tests; production builds always use the Rust core.

Exit gate: create two sessions, search/filter terms, open tabs, restart, and recover the correct session and tab state without a remote write.

### M2 — editing, tabs, and durable autosave

Deliver:

- WYSIWYG body editing, alternative-definition support, minimal toolbar, outline, and safe Source view.
- Details inspector for title, description, grouped tags, and diagnostics.
- Persistent tab order/pinning, overflow, keyboard navigation, and correct close/reopen behaviour.
- Revision-checked autosave, save/error indicators, flush-on-transition, local revision recovery, and explicit per-term revert.
- Source-preserving serialization and minimal-diff checks against real fixtures.

Exit gate: edit ten terms across tabs, close several tabs, restart and force-terminate in separate tests, and recover every acknowledged edit with no unrelated file changes.

### M3 — review and first GitHub submission

Deliver:

- Session-wide readable/source comparisons and the contribution title/description form.
- Production authentication UI and account binding.
- Head-repository selection, fork creation/readiness, verified baselines, multi-file upload, stable branch identity, and PR creation.
- Durable operation journal and retry behaviour for response loss at branch/commit/PR stages.
- Progress and errors that preserve the local editing session.

Exit gate: a contributor without upstream write permission submits ten edited term files in exactly one PR. A retry after an ambiguous response links that same PR and produces no duplicate contribution.

### M4 — resume, reconcile, and interchange

Deliver:

- List/import the signed-in user’s existing PRs and reopen locally linked sessions.
- Distinguish total proposed changes from newer local changes.
- Update an existing PR branch with concurrency detection and no force push.
- Three-way reconciliation UI for supported cases; a safe retained-draft state for unresolved complex conflicts.
- Versioned session export/import and local CLI operations for agents.
- Explicit merged/closed handling and follow-up-session creation.

Exit gate: reopen a PR on a fresh installation, edit another term, and update the same PR. Remote edits made independently must be preserved or surfaced as conflicts. An agent-prepared session imports without requiring the agent to publish or handle credentials.

### M5 — expert trial and platform release candidate

Deliver:

- A focused trial with representative experts using their team filters and real terminology examples.
- Keyboard, screen-reader, contrast, reduced-motion, and text-scaling checks.
- Packaged smoke tests on macOS, Windows, and Linux webviews, with secure-storage behaviour checked per platform.
- Installation instructions, release notes, license/notices, OAuth setup documentation, and local recovery/export documentation.
- CI for Rust core tests, frontend checks, fixture integrity, UI flows, and platform builds.

Exit gate: the end-to-end release scenario below succeeds on the supported platform matrix. Signing and public distribution are release tasks contingent on configured maintainer credentials; an unsigned local development build is not described as a finished signed release.

## 3. Release acceptance scenario

An expert who has not used Git can:

1. Install and open the application without installing Node, Rust, Git, or GitHub CLI.
2. Authenticate with their own GitHub account.
3. Filter to their team and a discussion status, then find a specific term.
4. Open at least three term tabs and edit at least ten terms in one session.
5. Edit one source list and one alternative definition without changing unrelated content.
6. Close a tab, restart the app, and recover all acknowledged drafts and the expected workspace.
7. Disconnect the network, edit another term, and see successful local saving.
8. Reconnect, review the contribution, and submit exactly one PR.
9. Return after feedback, make corrections and add another term, and update the same PR.
10. Export the session and open it on a second installation without leaking credentials.

The PR diff contains only intended term changes. The contributor never needs to choose Git commands, resolve raw conflict markers, or manually create a fork.

## 4. Tests that address material risks

| Area | Meaningful checks |
|---|---|
| Source integrity | Every corpus file opens/closes without change; metadata-only edits leave body bytes intact; paragraph edits preserve unrelated blocks; unsupported syntax survives |
| Scientific structure | Multiple definitions, numbering, empty headings, sources, tables, mathematical/scientific notation, and Unicode remain present |
| Tabs | Duplicate opens focus the same buffer; undo belongs to the active term; rapid switching cannot cross-write terms; closure keeps drafts |
| Persistence | Force termination after acknowledged save; out-of-order responses; stale revision rejection; migration rollback; disk-full/write-denied recovery |
| Search | Title/synonym ranking, filter OR/AND semantics, changed-only results, and keyboard focus behaviour |
| Session isolation | Same term in two sessions; switching with pending saves; importing a session already linked to a local PR |
| Authentication | Authorization denied/expired/cancelled; token revoked; different account; organization restriction; unavailable secure store |
| First submission | Maintainer and ordinary contributor routes; fork delay; multi-file atomic commit; no unrelated path changes |
| Recovery | Lost response after branch creation or PR creation; restart mid-upload; retry does not create another PR |
| Concurrency | Independent PR-head edit; upstream change to the same term; deleted file; branch updated between preflight and reference update |
| Submission snapshot | Continue typing while upload runs; only frozen revisions are marked submitted; newer edits remain local |
| Public-session import | Fresh installation; authored PR ownership checks; merged/closed PR; non-term changes explicitly shown |
| Privacy | Issued tokens never return to the frontend; optional PAT input is cleared after its dedicated IPC transfer; no credentials in logs, exports, or diagnostics; rendering cannot execute scripts |
| Native runtime | Packaged WebKit/WebView2/Linux behaviour, credential store, native browser opening, app exit and resume |

Use mocked GitHub transport and failure injection for deterministic core tests. Use a dedicated test repository and ordinary contributor account for controlled integration tests; do not exercise destructive scenarios against the glossary’s production repository.

UI tests should cover the complete journeys and accessibility semantics. Snapshot tests of incidental CSS classes and tests that merely repeat implementation details are not priorities.

## 5. Initial performance targets

These are product targets to measure on documented representative hardware, not existing benchmark results:

- Cached workspace becomes interactive within about 1.5 seconds after the native window opens.
- Search/filter results update within 100 ms for the current corpus and a 10,000-term synthetic catalogue.
- A typical term opens within 150 ms from local cache.
- A normal idle save is acknowledged within 750 ms of the last keystroke; continuous editing persists at least every two seconds when storage is healthy.
- Thirty open tabs remain usable without mounting thirty full editors or blocking typing.
- Network loading/submission runs off the UI thread and exposes cancellation or a safe background state.

Measure on native webviews. Adjust targets using evidence without weakening the source-preservation and local-durability guarantees.

## 6. Release and maintenance

Target macOS Apple Silicon/Intel, Windows x64, and Linux x64 initially. Validate these choices with actual expert machines before expanding to more architectures or installer formats. Platform minimum OS/webview versions are set from the selected Tauri/editor dependency requirements during scaffolding.

Use a single package manager and check in its lockfile plus `Cargo.lock`. Pin desktop/security-sensitive dependencies deliberately. Review dependency and native-binary notices before distribution.

A public release needs a repository location, chosen license, project OAuth app registration/client ID, and platform signing configuration. These are explicit setup items; the application itself does not require hosted infrastructure.

For the first trial, distribute a documented local development/release-candidate build. Automatic updates can follow a verified signing/update pipeline rather than being a prerequisite for testing the editing workflow.

## 7. Decisions retained for later

- Actual supported expert OS/architecture mix and institutional installation restrictions.
- Whether team preferences should be suggested during onboarding or remain entirely optional.
- New-term creation is now included by explicit user instruction, with the full skeleton displayed immediately.
- Whether a future reviewer mode should submit formal GitHub approvals and how that maps to CEOS governance.
- Whether a `gh` adapter becomes justified after the direct-integration experiment.

These do not change the confirmed stack, local-first operation, three main entry points, tabs, autosave, or one-session/one-PR requirement.
