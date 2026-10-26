# Architecture and data integrity

## 1. Runtime and boundaries

```text
Tauri native window
┌─────────────────────────────────────────────────────────────┐
│ Svelte 5 + TypeScript + Tailwind 4                            │
│ Search · Sessions · Tabs · WYSIWYG · Review                   │
│ Transient presentation/editor state                         │
└──────────────────────────┬──────────────────────────────────┘
                           │ typed Tauri commands + events
┌──────────────────────────▼──────────────────────────────────┐
│ Rust application core                                      │
│ Catalogue · Session service · Source codec · Validation      │
│ Autosave · Submission coordinator · GitHub client            │
├───────────────────┬─────────────────────┬───────────────────┤
│ SQLite + snapshots│ OS credential store  │ Direct HTTPS      │
│ and local exports │                     │ to GitHub         │
└───────────────────┴─────────────────────┴───────────────────┘
```

Bundle a static Vite application into Tauri 2. Use ordinary Svelte components and a small feature-level state layer. A server-rendered SvelteKit deployment is unnecessary for this desktop application. Do not add an internal HTTP API when Tauri IPC already covers the frontend/backend boundary.

Rust is the authority for source snapshots, local drafts, identity, validation, filesystem operations, and GitHub mutations. Device-flow tokens are never returned to the frontend. The UI cannot execute arbitrary shell commands or write arbitrary filesystem paths. An optional manually entered PAT has the explicit transient-input exception described in the authentication section.

Initial dependencies: `serde`, `tokio`, `reqwest`, `rusqlite` with an embedded SQLite build, a maintained native credential-store adapter, Markdown parsing/diff libraries, and Tauri’s narrowly scoped native integration. Select exact packages and compatible locked versions during the first implementation milestone. Avoid adding several competing HTTP, state, or database frameworks.

## 2. Source corpus and snapshot loading

Configure the initial repository as `ceos-org/eo-glossary`, content root `docs/terms`, and the repository’s actual default branch. Store repository identity and the chosen base commit in every session; never treat a mutable branch name as a complete baseline.

Initial load:

1. Resolve repository metadata and the default branch commit.
2. Download an archive at that immutable commit, with bounded archive/download size.
3. Extract only ordinary Markdown files under the configured content root into application-owned storage. Validate paths and reject links, traversal paths, unexpected nested entries, and oversized files.
4. Parse metadata and body sections, recording the original bytes and source hashes.
5. Build a local search catalogue and publish it to the UI atomically.

An archive avoids making hundreds of individual requests to fetch small term files. Validate GitHub redirect hosts and credential forwarding. Do not execute repository scripts or configuration, extract into a user checkout, or render raw repository HTML with privileges.

Refresh resolves a new immutable snapshot. On opening and before submission, the active session reconciles against the latest fetched main. Non-overlapping edits merge automatically; overlapping edits retain both sources and block publication until the author chooses. A frozen upload is recovered before reconciliation. Each choice checks both draft revision and remote snapshot identity. The UI displays the snapshot date and offline state. All term contents needed for the current catalogue are cached, making offline editing practical.

For the initial 509-term corpus, a normalized in-memory search index is sufficient. Index title, synonyms, definitions, and selected notes; keep sources searchable at a lower rank. Rebuild affected entries after saved edits. Use SQLite FTS only if measurements justify it.

## 3. Domain model

| Object | Important fields |
|---|---|
| Repository | stable ID, host, owner, name, default branch, content root |
| SourceSnapshot | repository ID, base commit, fetched time, catalogue version |
| SourceFile | relative path, exact bytes, Git blob/source hash, parsed metadata, diagnostics |
| Session | UUID, name, repository, base snapshot, owner account ID when linked, note, timestamps, archive flag |
| SessionTerm | session ID, path, starting source, current draft, local revision, last submitted source/revision, optional checked flag |
| WorkspaceState | session ID, tab order, pinned tabs, active tab, filters, selection/scroll anchors |
| SubmissionLink | session ID, upstream repository, head repository/branch, PR number/URL, last known head commit |
| SubmissionOperation | UUID, session ID, frozen term revisions, expected remote head, stage, resulting commit/PR, retry information |
| LocalRevision | session/path, monotonically increasing revision, source content, timestamp, editing origin |

Keep three versions conceptually separate:

- **Starting source:** the term as it was when this session began, or the reconciled baseline after a deliberate refresh.
- **Last submitted source:** the term revision most recently sent to this session’s PR.
- **Current draft:** the latest durably saved local edit.

This supports both “all proposed changes” and “changes not yet submitted.” A successful upload changes the submitted pointer, not the current editor buffer. A session with no local changes can still contain an open PR with many proposed changes.

The stable term identity is its repository path. Titles and tags are editable attributes. Tabs are keyed by session and term identity, never only by title.

## 4. Session and operation states

Do not flatten every condition into one status enum. Store these dimensions independently:

| Dimension | Values |
|---|---|
| Submission linkage | Local only / linked PR |
| Remote PR lifecycle | Unknown / open / closed / merged |
| Draft synchronization | Matches last submission / newer local changes / remote changes / conflict |
| In-flight operation | Idle / preparing / uploading / linking PR / reconciling / failed |
| Local durability | Saved / pending / failed |

For example, “PR open + newer local changes + saved locally” is a normal state. It should not be presented as a failed or unpublished session.

Important transitions:

- First edit creates or updates a local session term.
- Local autosave increments a revision in a transaction.
- Submit freezes acknowledged revisions and writes an operation intent before network mutations.
- Successful upload advances last-submitted pointers only for the frozen revisions.
- Remote state checks update PR status without discarding drafts.
- Merge makes the linked contribution historical; continuing starts a new session from current upstream.
- Archiving affects only local visibility. It does not delete a remote branch or close a PR.

## 5. Local persistence and autosave

Store the database and source snapshots under the platform’s application-data directory. Use SQLite transactions and WAL where supported. Keep blocking database work away from the UI thread and serialize writes through a small persistence service.

The frontend initially debounces edits for 500 ms, with a maximum two-second interval during continuous typing. Each save carries:

```text
session_id
term_path
expected_revision
client_sequence
proposed_source / structured patch
```

Rust validates identity and source integrity, applies the update in a transaction, then returns the acknowledged revision and content hash. A stale revision is rejected or reconciled explicitly. Older responses cannot replace newer UI content or set a newer edit to “Saved.”

Per-term write queues collapse redundant intermediate pending edits while preserving the latest content. Source-mode and metadata writes use the same revision mechanism; they cannot race independent body saves.

On tab close, session switch, explicit Cmd/Ctrl+S, and orderly application exit, flush outstanding work and wait for acknowledgment. Closing a tab is separate from reverting its saved draft. Use a Tauri single-instance guard initially so two app processes cannot independently edit the same local session.

Guarantees:

- Every acknowledged revision survives process termination under normal filesystem durability assumptions.
- Unacknowledged keystrokes are shown as pending; the UI does not promise they are already durable.
- A failed write leaves the editor content recoverable and keeps the error visible.
- A frozen submission uses durable revisions only.
- Database migrations preserve drafts and create a recoverable backup before destructive schema changes.

Keep a bounded history of local revisions and explicit checkpoints. Define retention and disk limits before release; never prune the latest draft, starting source, last-submitted source, or revisions referenced by an in-flight operation. Export and backups use consistent database snapshots, not an unsafe copy of an open WAL database.

## 6. Markdown and YAML integrity

### Editor selection

Start with Milkdown because its architecture is centred on Markdown, ProseMirror, and remark. Mount it behind an `EditorAdapter` interface in Svelte and use only the editing features needed for the glossary. Crepe is an integration candidate, not a requirement to accept its entire default theme.

Evaluate Tiptap as the fallback if Milkdown fails integration or source-preservation requirements. Its official Markdown extension is currently documented as beta. Neither candidate should be assumed to preserve the corpus merely because it can parse Markdown.

### Source-preserving codec

Keep the full original file as source of truth. Split frontmatter and body using a parser that records source ranges; do not identify YAML boundaries or Markdown structure with broad regular-expression replacements.

The codec should:

1. Retain exact original frontmatter bytes and unknown fields.
2. Parse the body into recognized structures while retaining raw source ranges for blocks.
3. Associate editor nodes with source blocks and record which blocks were actually changed.
4. Reuse original source for untouched blocks; serialize only changed blocks or explicitly structural edits.
5. Rebuild an entire changed structure, such as a table or list, only when necessary, and show that in the review diff.
6. Apply metadata edits to the affected YAML field spans rather than rewriting the entire frontmatter.

The first experiment must determine how much source-range bookkeeping the selected editor needs. If arbitrary block preservation proves too fragile, narrow rich editing to recognized definition/notes/examples/source sections and retain unsupported ranges verbatim. Do not ship blanket document normalization as an invisible workaround.

### Hard compatibility cases

- Multiple numbered definitions and separator lines such as `___`.
- Repeated title headings before alternative definitions.
- Empty definition and section headings.
- Quoted multiline descriptions, different indentation, YAML comments, and additional metadata.
- Scientific symbols, Unicode, superscripts/subscripts, escaped punctuation, and embedded HTML.
- Tables, nested lists, reference-style links, source URLs, code, and mathematics where present.
- Existing malformed or ambiguous content, which should remain readable and diagnosable.

Opening, focusing, changing tabs, or switching views must not create a change. Editing only tags must leave the body byte-identical. Editing only a paragraph must preserve frontmatter and unrelated body blocks. Unsupported constructs must remain represented or block destructive conversion.

Validation distinguishes pre-existing diagnostics from newly introduced damage. Incomplete drafts always save locally. A submission can carry existing content warnings, but cannot drop a frontmatter field, remove unsupported content invisibly, or produce an invalid file through a codec failure.

## 7. GitHub authentication

Use a project-owned OAuth app with device flow enabled. Ship only its public client ID. Rust requests a device code, emits the user-facing code and verification URL to the UI, polls GitHub at the required interval, and verifies the resulting account before storing credentials.

Handle pending authorization, cancellation, expiry, access denial, slower polling requests, transport failures, and organizational restrictions explicitly. The user can copy the code and open GitHub manually if automatic browser opening fails.

For the public-glossary scope, request only permissions required to read content, write the user’s eligible head repository, and open/update PRs. Validate the `public_repo` OAuth route in the authentication experiment; do not silently broaden to private repositories. A supported PAT can be an advanced fallback, with permission errors explained in context. Fine-grained tokens and fork access need their own compatibility checks.

If the fallback uses an in-app password field, the manually entered PAT necessarily exists briefly in that input and in one dedicated IPC request to Rust. Clear it immediately after transfer, keep it out of shared/reactive workspace state and diagnostics, and never return it from Rust. This exception does not apply to tokens issued by the normal device-flow sign-in.

Tokens and any refresh tokens remain behind a `CredentialStore` interface using the OS credential vault. Do not fall back to plaintext automatically. If secure storage is unavailable, offer an explicit session-only sign-in and explain that the user will sign in again after restart.

Account identity is a stable GitHub user ID, not only a display login. Bind remote session operations to that identity. Signing out does not delete drafts; reconnecting as a different account does not grant ownership of another account’s PR-linked session.

No OAuth client secret, repository-wide shared token, hosted callback, or application server is needed for this design. The exact device-flow requirements are documented by GitHub in the references below.

## 8. GitHub client and one-PR submission

Use an explicit Rust GitHub client backed by `reqwest` for the required REST endpoints. Keep HTTP transport, credentials, and domain-level submission coordination separate enough for deterministic tests. Add GraphQL only if it materially simplifies an established query. Pin a supported API version and handle pagination, rate-limit responses, request IDs, and timeouts.

No local Git checkout is required. Use Git data APIs to create blobs, trees, commits, and references. One commit can contain every term in the frozen submission. Retain unrelated files by using the correct complete base tree.

### First submission

1. Flush and validate local drafts; freeze the exact selected revisions.
2. Persist a `SubmissionOperation` with a stable session-derived branch name and intended upstream/head repositories.
3. Resolve current upstream and check changes to the session’s edited files since its starting snapshot.
4. Reconcile non-overlapping changes. Pause for a readable conflict decision when both sides changed overlapping content or the remote deleted/renamed a term.
5. Select an eligible head repository. An existing personal fork works for ordinary contributors; upstream branch creation can be used when permitted and appropriate. Handle disabled forking and denied permissions without losing drafts.
6. If creating a fork, wait for GitHub to make it ready. Verify the selected base commit/tree is available in the head repository. Test fork-network object availability and use an explicit synchronization/import strategy when needed; never assume a stale fork is current.
7. Upload the changed term blobs and a tree based on the verified baseline, then create one commit. Leave all unedited paths intact.
8. Create the stable session branch or update the already reserved branch through a safe fast-forward operation.
9. Search for an existing PR with this exact head/base before creating one. If it exists, link it; otherwise create the PR with the reviewed title and body.
10. Persist the PR number, head commit, and submitted revision pointers, then return the actual GitHub URL to the UI.

Branch names include a session UUID, not only a title. A user-readable session name can change without losing the submission link. No branch is created during ordinary local saving.

### Updates to an existing submission

Verify that the PR remains open, targets the expected repository, and is eligible for this account. Read its latest head commit before creating an update. If the head changed, reconcile against that new source rather than overwriting it.

Create a child commit of the verified expected head, using that head’s tree and only the intended term changes. Update the branch without force. If a concurrent commit makes the update non-fast-forward, stop and reconcile again. A preflight check alone does not provide concurrency safety.

Keep the existing PR number. An update must not create another branch/PR because the original operation timed out or local metadata was temporarily unavailable.

### Retry and crash recovery

The operation journal records stage changes before/after network mutations. On restart or an ambiguous response, inspect the stable remote branch, resulting commit/tree, and exact matching PR before retrying a write. For each frozen snapshot, record a content fingerprint so identical work can be recognized without blindly appending commits.

Uploading immutable blobs/trees can be retried by content identity. A branch or PR may already have been created even if the response was lost. Recovery discovers and links that object. Never solve this by creating a new randomly named branch on every retry.

## 9. Resuming public sessions and conflicts

List authored PRs through GitHub, following pagination and validating the configured upstream. Import base/head content at immutable commits and identify the changed term paths. On a machine with no local metadata, reconstruct an editing session from the PR’s base and head, with the head as the last-submitted state.

If a PR already has a local session link, open that session. If the PR contains non-term changes, make this visible and initially limit import to read-only inspection until their preservation is guaranteed. Do not hide unrelated changes in a contribution that the user is about to update.

Use three-way comparison for draft reconciliation. Handle both current upstream changes and independent changes on the PR head. An upstream branch merge conflict is distinct from a local-draft versus PR-head conflict; surface the relevant resolution without rewriting remote history.

V1 does not force-push, merge PRs, delete remote branches, or reopen closed PRs automatically. Where safe updating needs a more complex merge, retain local changes and expose the exact limitation rather than pretending synchronization succeeded.

## 10. IPC contracts and frontend state

Representative command families:

```text
auth_status / auth_start_device / auth_cancel / auth_sign_out
catalogue_load / catalogue_refresh / terms_search / term_open
session_list / session_create / session_rename / session_open
draft_save / draft_revert / draft_history / workspace_save
session_export / session_import
submission_preview / submission_start / submission_retry / submission_status
github_pr_list_authored / github_pr_import
```

Events include authentication progress, catalogue progress, save acknowledgements, remote status changes, and submission progress. Use shared/generated TypeScript types for command payloads and results. Return structured error codes with actionable messages; never return raw credential-bearing HTTP responses.

The Svelte state layer tracks the active session, open tab descriptors, active editor state, search/filter state, and acknowledged/pending revisions. It does not independently implement submission state transitions or keep an alternative durable database in localStorage.

Async results are checked against session identity, term identity, and request generation before being applied. A slow response from a previous tab/session cannot populate the current editor.

## 11. Session interchange and agents

Define a versioned `.eo-session.json` document, containing:

```json
{
  "schemaVersion": 1,
  "session": { "id": "uuid", "name": "WGCV terminology review" },
  "repository": {
    "host": "github.com",
    "owner": "ceos-org",
    "name": "eo-glossary",
    "contentRoot": "docs/terms",
    "baseCommit": "immutable-commit-id"
  },
  "terms": [
    {
      "path": "docs/terms/accuracy.md",
      "startingSource": "complete original Markdown",
      "draftSource": "complete proposed Markdown",
      "revision": 4
    }
  ],
  "submission": null,
  "workspace": { "openTerms": ["docs/terms/accuracy.md"] }
}
```

The normative schema also includes source hashes, timestamps, compatibility metadata, and last-submitted source when relevant. The export contains the data needed to compare a changed term offline, not merely pointers to a cache on the original computer. Read-only tabs without bundled sources can be restored after catalogue loading.

Exclude all credentials and device-flow codes. Validate schema version, repository identity, content paths, source lengths, and hashes on import. Imported remote links are claims to verify before enabling mutation.

Provide a small Rust CLI sharing the application core for listing terms, creating a session, setting an expected-revision draft, validating, and exporting/importing sessions. Local CLI operations do not publish to GitHub. An agent can prepare a portable session file or operate through this interface; it does not edit SQLite internals or need token access. Multiple local writers use the same revision checks and database locking.

## 12. Application security and packaging

- Use Tauri capabilities with a narrow command surface. No broad shell or arbitrary frontend filesystem permissions.
- Treat repository Markdown, imported sessions, and PR text as untrusted data. Disable executable HTML/MDX in rich editing, sanitize rendering, and retain unsupported source safely.
- Open validated HTTPS source/GitHub links externally. Prevent arbitrary command or custom-URL execution from document links.
- Restrict network credentials to the intended GitHub host; do not forward authorization across arbitrary redirects.
- Bound archive extraction, import size, and image/resource handling. Do not execute package scripts from the glossary repository.
- Keep tokens out of logs, crash reports, diagnostics, and JSON exports. Never copy issued tokens to the clipboard. Do not upload document content to analytics or AI services.
- Prefer explicit release downloads first. Add automatic updates only with a configured signature-verification and release process.

Package the frontend, Rust backend, and required native resources into platform installers. Users need no Node, Rust, Git, or `gh` installation. Build/test on actual platform webviews; a successful Chromium-only test does not establish macOS WebKit or Windows WebView2 compatibility.

## 13. Planned source layout

This is the implementation target, not a claim that these modules already exist:

```text
src/
  app/                  shell, navigation, command routing
  features/
    search/             catalogue UI and filters
    sessions/           local/remote session lists
    editor/             tabs, EditorAdapter, metadata, source view
    review/             comparisons and submission UI
    auth/               device-flow presentation
  lib/ipc/              typed Rust command bindings
  lib/ui/               small accessible controls
  styles/               tokens and editor prose styling
src-tauri/
  src/                  native entry point and Tauri commands
  capabilities/         explicit application permissions
crates/editor-core/
  src/
    catalogue/          snapshots and local search
    source/             Markdown/YAML codec and diagnostics
    sessions/           revisions and lifecycle
    storage/            SQLite and migrations
    github/             device flow, HTTP client, credentials
    submissions/        remote coordination and recovery
crates/editor-cli/      local agent/session operations
tests/fixtures/         corpus cases and expected source changes
tests/e2e/              expert workflow tests
```

Use a shared Rust core to keep domain tests and the agent CLI independent of the Tauri window. Avoid splitting the application into more crates until it improves a concrete dependency boundary.

## 14. Primary references

Checked on 26 October 2026. The architecture above is a proposed design; the references establish the relevant platform/API capabilities.

- [Tauri trust boundaries and security](https://v2.tauri.app/security/).
- [Tauri sidecar packaging](https://v2.tauri.app/develop/sidecar/).
- [Svelte documentation](https://svelte.dev/docs/svelte/overview).
- [Tailwind CSS with Vite](https://tailwindcss.com/docs/installation/using-vite).
- [Milkdown source and architecture](https://github.com/Milkdown/milkdown).
- [Tiptap Svelte integration](https://tiptap.dev/docs/editor/getting-started/install/svelte).
- [Tiptap Markdown extension and current beta status](https://tiptap.dev/docs/editor/markdown).
- [GitHub OAuth device flow](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow).
- [GitHub trees](https://docs.github.com/en/rest/git/trees), [references](https://docs.github.com/en/rest/git/refs), and [pull requests](https://docs.github.com/en/rest/pulls/pulls).


## Local implementation notes

`editor-core` contains source parsing/mutations, SQLite storage, the credential-free command boundary, and GitHub transport. The Tauri process owns credentials and submission orchestration. `eo-editor-cli` provides local session/agent commands. Sectional rich editors are installed with a ProseMirror plugin before the view becomes editable; synchronous state changes feed an ordered revision-aware save queue. Required heading lines stay outside section editors. Only edited fragments are serialized by Milkdown; the Rust source parser patches their byte ranges. The complete catalogue is pinned per session and included in portable exports. An optional, feature-gated loopback CLI bridge exists solely for real-core browser workflow tests, and production frontend code cannot select it. Actual tested behavior and remaining release gates are in `VALIDATION.md`.


Repository settings select the catalogue used for new sessions. Session repository identity remains immutable. Snapshot storage uses a composite repository/SHA key and PR lookup uses repository/number. The credential-free source renderer supplies structured document sections to the review UI; browser HTML sanitization is applied at the final rendering boundary and word highlights preserve the rendered element tree. Switching repository downloads and validates its default-branch snapshot before an atomic local catalogue/session change.


## Main updates and workspace recovery

The native process checks the selected repository's actual default-branch head at every opening. Cached immutable snapshots avoid downloading an unchanged archive. The core applies a new baseline and three-way draft merges in one immediate SQLite transaction, writing earlier changed drafts to revision history. Conflicts store their original common ancestor, remote source/deletion, and remote commit. Further updates preserve that ancestor. Resolution checks local revision and remote commit before choosing a version; newer updates invalidate stale choices. Pending submission journals defer reconciliation. Public PR state is checked alongside main so merged/closed sessions become read-only without losing in-flight local saves.

Native geometry is persisted by Tauri window-state. Zoom is a validated global SQLite preference applied to the native WebView. Session workspace JSON stores tabs, filters, page, per-term panels/source modes, scroll offsets, review mode, contribution text, and non-secret form drafts. Session selection opens editing; startup restores the last page. The frontend gates navigation during the brief transactional apply, flushes saves, and reloads the merged source. An unchanged main check avoids remounting editors. Credentials, device codes, PAT inputs, and rich undo/caret state are excluded from workspace persistence.

Authentication supports GitHub Apps and legacy OAuth apps. GitHub App permissions come from registration; OAuth requests `public_repo`. Expiring device-issued tokens rotate through GitHub directly, serialized in Rust, with refreshed credentials kept in the OS vault when Remember me is enabled. No client secret, app private key, sidecar, or hosted auth service is used. See Decision 002 and GITHUB-SETUP.md.
