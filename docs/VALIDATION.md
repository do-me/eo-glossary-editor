# Local validation, 26 October 2026

This is a working local development build, not a signed cross-platform release.

## Automated checks

- Rust source-integrity tests run against all **509** bundled term files at glossary commit `62edc6225cee8219b217f5cbacda2830ab48dc98`.
- No-op opening/section editing preserves exact source. Description-only edits preserve the body. Unknown YAML fields and custom tags survive metadata changes.
- Section edits preserve surrounding bytes and the existing heading sequence. Title/required heading replacement, heading insertion, setext-heading insertion, and definition-separator insertion are rejected by Rust.
- New-term scaffolds and alternative-definition scaffolds include the required empty sections.
- SQLite tests cover reopening, two independent writers, stale-revision rejection, session isolation, export/import, new-term collision/revert, and forced process termination after a durable save acknowledgement.
- HTTP tests cover ordinary-contributor fork creation, one tree/commit for multiple files, updates to an existing PR without force push, remote-head refusal, PR response-loss recovery, operation-journal restart, and continued local edits during a frozen submission.
- TypeScript tests cover ordered revision acknowledgements, write coalescing, retained/retried write failures, and search/filter semantics.
- Playwright tests use the actual credential-free Rust command boundary and SQLite database. They cover new-term previews, protected headings, WYSIWYG editing, metadata autosave, tab closure/reopening, reload, independent sessions, session review, rapid switching, and a source-edit attempt to replace a protected heading.
- Svelte/TypeScript checks, ESLint, Prettier, rustfmt, Clippy with warnings denied, and dependency audit are part of the local check set.

The ignored `crash_writer_child` test is a subprocess fixture used by the forced-termination test, not an untested product case.

## Native check

Built an optimized unsigned macOS Apple Silicon `.app` (13.99 MiB), launched it, checked native Quit/relaunch, and inspected the actual native webview/accessibility tree and screenshot. Search results, a term tab, definition/notes/examples/sources, alternate definitions, local-save feedback, and review count render in the native runtime. The app's local data is separate from the existing glossary checkout.

## Explicit remaining release validation

- A registered GitHub App (recommended) or OAuth app/public client ID must be configured to exercise device authorization end to end. No third-party app identity is borrowed.
- No credentials were entered into this app during implementation. Device denial/expiration/keychain behavior has code paths but still needs a real-account and OS credential-store trial.
- Remote writes were mocked. No production PR/fork/commit was created to test this app. Live organization/SSO/fork-readiness and retry behavior needs a dedicated sandbox integration trial.
- Main-versus-draft conflicts have rendered comparison, source diffs against their common ancestor, and explicit version choices. Conflicts with independently changed PR branches still stop submission and require GitHub reconciliation. Removing an already submitted file after its deletion on main also needs a GitHub-side PR edit; keeping it locally can propose restoration.
- Milkdown normalizes the edited section. Untouched sections remain byte-for-byte intact. Unsupported source syntax stays in source editing; it is not silently converted.
- Rich undo history belongs to the currently mounted editor; saved content persists when tabs are unmounted.
- Windows/Linux packaging, signed/notarized distribution, screen-reader trials, and user trials remain release work. CI is supplied; platform results are recorded in the source repository’s GitHub Actions runs. Local tests do not establish Windows/Linux compatibility.


## Final local results

27 Rust integration tests passed (plus one intentionally ignored subprocess fixture), 8 TypeScript tests passed, and 9 browser workflow tests passed. The restart/zoom workflow also passed with an explicit reading-position assertion after reload. The corpus check exercises every one of the 509 term sources. Svelte checking reports zero errors/warnings; ESLint, Prettier, rustfmt, Clippy with warnings denied, and npm audit pass (zero reported dependency vulnerabilities). The optimized native build and native quit/restart smoke test pass. Browsing a term after the final restart retains the exact acknowledged source revision and produces no additional saved edit.


## Repository settings and rendered review update

Added repository URL settings, transactional repository switching, repository-scoped snapshots/PR lookup, and a version-1-to-2 SQLite migration. Existing sessions and interrupted uploads retain their destination; the session header and submission screen identify it. Rendered review offers Changes, Preview, and Side by side, including word-level highlights, formatted citations/lists/tables, tag and description changes, and a collapsed source diff. Rendering does not serialize or change saved Markdown. All rendered source HTML passes through a shared sanitization boundary; scripts, unsafe links, event handlers, and arbitrary UI styling are removed.

Additional coverage includes repository URL validation, same-SHA fork isolation, import/export and follow-up isolation, legacy database migration, identical PR numbers across repositories, fork-specific PR creation/update routing, safe HTML rendering and nested word highlights, repository switching through the UI, and all three rendered review modes. The test-only loopback transport supplies a deterministic repository fixture for settings tests; production repository settings always download GitHub data.

The updated optimized macOS app loaded `do-me/eo-glossary` through the real GitHub download path and retained that selection after a native quit/restart. The existing session and its two drafts were preserved. The repository is selected locally for the user’s end-to-end authentication/commit/PR trial; no remote writes were performed during this update. Added empty definition skeletons are identified as structural additions in review, even when their bodies are blank.


## Zoom, restart, main synchronization, and authentication update

- Rust tests verify non-overlapping main merges, unchanged-head idempotence, conflicting-source retention, publication blocking, both explicit choices, stale revision/remote-head rejection, durable overwritten revision history, new-term collisions, remote deletion, submitted-deletion preservation, and zoom/workspace storage. Device-request tests distinguish GitHub App registration permissions from OAuth `public_repo` scope.
- Browser workflows exercise startup main updates through the real Rust/SQLite command boundary with deterministic simulated merged PRs. Both conflict choices survive restart; independent main/local changes combine. Zoom shortcuts, panel/source/filter state, reading position, review mode, and unfinished contribution text restore after reload.
- Native release and debug bundles build on this Mac. The release loaded CEOS through real GitHub, reported main checked, retained the fork session's Ad-hoc Order edit and the two earlier CEOS drafts, and restored the selected session, Accuracy tab/search, Details panel, and 110% zoom after quit/relaunch. Window geometry was saved in Tauri's window-state file. After the Mac was unlocked, native Ctrl-plus increased zoom from 115% to 125%, and Ctrl-minus restored 115%. Two normal quit/relaunch cycles retained 115% zoom, the active term/search/Details panel, and the exact visible reading position. Native geometry was verified unchanged after restart: 1427 × 1028 at (998, 249), with maximized/fullscreen flags false. The updated editor was left open. Browser keyboard and scroll recovery tests also pass.
- `gh` read-only checks confirm `do-me` is repository admin and an ordinary CEOS organization member. No registration client ID is configured. No GitHub app was created/installed, no organization permissions changed, and no production or sandbox remote write occurred.
- GitHub App device authorization, user-token refresh without a client secret, OAuth compatibility, and native credential storage are implemented. Live consent, expiring-token rotation, installation/fork permissions, and commits/PRs still need the user's sandbox trial with a registered app.

CEOS is now the locally selected default repository. The test fork remains available through settings and its original session keeps its destination. Cached editing remains usable without a successful startup network check.


## Source publication

The editor source is published as `do-me/eo-glossary-editor`. Authentication registration belongs to the personal `do-me` account, with that source repository as the homepage. The default glossary data repository remains CEOS; the sandbox data repository remains `do-me/eo-glossary`. No GitHub App registration, installation, or live editor-authenticated term commit/PR has been performed as part of publishing the source.


## First release and submitted-PR review, 27 October 2026

The maintainer reports successful personal-token authentication, commits, and PR creation in the glossary test fork. This is a user-reported live result; device authorization remains untested.

The submitted review now has one persistent PR summary, a full-contribution view by default when nothing remains to send, a separate Not submitted scope, submitted/pending counts, one primary PR link, and a Continue editing action. Merged/closed sessions offer follow-up and cannot display an active submission form. Export, all rendered comparison modes, source diffs, interrupted-operation recovery, and same-PR updates remain available. Success scrolls/focuses the summary. A real-core browser workflow exercises submission acknowledgement, restart, scope changes, later edits, same-PR updates, and merged follow-up. All nine browser workflows pass. The native release was checked with the maintainer's existing PR #1: two submitted terms render in the full-contribution view, no pending edits or duplicate success messages appear, and the existing account/session remain intact.

The release workflow gates publication on quality checks and packaging for macOS Apple Silicon/Intel, Windows x64, and Linux x64. It keeps a draft until every build succeeds and all seven required installer formats are present, then attaches SHA-256 checksums and source notices before publication. macOS is ad-hoc signed without notarization; Windows is unsigned. Windows/Linux builds are verified in CI; interactive use on those operating systems still needs user trials.
