# EO Glossary Editor — product and implementation plan

Date: 26 October 2026
Status: implemented local development build; see `docs/VALIDATION.md` for verified coverage and release gaps.
Source repository: [do-me/eo-glossary-editor](https://github.com/do-me/eo-glossary-editor)

## 1. Objective

Give CEOS subject-matter experts a small desktop application in which they can find relevant glossary terms, edit several terms comfortably, stop and resume, review the resulting changes, and submit one coherent contribution to GitHub.

The application should feel like a document editor with a manageable reading list. GitHub supplies identity, repository storage, and review. The application handles the mechanics of branches, forks, commits, and PR creation.

The central product rule is:

> A session is a durable set of proposed changes to multiple term files. Opening a tab or saving a draft does not publish anything. Submitting the session creates one PR; later submissions update that same PR.

This plan implements the requirements in [glossary issue #13](https://github.com/ceos-org/eo-glossary/issues/13) and the subsequent decisions to use Tauri, Rust, Svelte, Tailwind CSS, tabs, and automatic local saving.

## 2. Confirmed decisions and working defaults

| Area | Direction | Status |
|---|---|---|
| Application | Custom Tauri desktop application | Confirmed |
| Backend | Rust | Confirmed |
| Frontend | Svelte + Tailwind CSS, simple and minimal | Confirmed |
| Editing | WYSIWYG, multiple term tabs, autosave | Confirmed |
| Main entry points | Search, sessions, term editing | Confirmed |
| Hosting | No deployed backend or authentication service | Confirmed |
| GitHub sign-in | Personal access token; app device flow optional | Confirmed primary route |
| GitHub integration | Direct Rust API client; no mandatory CLI sidecar | Recommended default; see decision record |
| Rich-text library | Milkdown, mounted through a small Svelte adapter | Implemented; section-preservation corpus checks pass |
| Local persistence | SQLite managed by Rust, with portable session export | Recommended default |
| Repository scope | CEOS glossary first, repository identity kept in the data model | Recommended default |
| Platforms | macOS, Windows, Linux; macOS first for interactive development | Planning assumption |
| Public code license | MIT is a sensible default; select before public distribution | Maintainer decision, not a blocker for local planning |

The application uses a static frontend bundled into its native window. It does not need an application web server, a database service, or a localhost HTTP server. SQLite is an embedded local file.

## 3. Experts and their work

### Primary user: domain expert

An expert knows the science, has a GitHub account, and contributes occasionally. They may review 5–30 related terms over several days. They should not need a terminal, Git installation, local checkout, or explanation of a branch.

Example: a WGCV expert filters to WGCV and “to be discussed,” opens Accuracy, Precision, and Uncertainty in separate tabs, checks the definitions against their sources, edits several paragraphs, and continues the next day. Once satisfied, they review the complete contribution and send one PR.

### Secondary user: returning contributor

After receiving feedback, the contributor opens the same submitted session, sees the linked PR and its state, makes corrections, adds another relevant term if needed, and updates the existing PR.

### Secondary user: maintainer or coding agent

A maintainer or agent can prepare a named session with a list of term identifiers and notes. The expert still uses the ordinary interface. A documented interchange format supports this without requiring an embedded AI service.

### Reviewers

The first release helps authors prepare contributions. Reading PR feedback and opening GitHub are supported. Formal PoC approval, reviewer assignment policy, and merging remain GitHub/governance responsibilities. A dedicated approval interface is a separate future feature.

## 4. Required user journeys

### A. First use

1. Launch the application.
2. See the CEOS glossary repository and the personal-token sign-in action.
3. Create a GitHub personal access token if needed and paste it into Settings.
4. Download a consistent glossary snapshot with visible progress and retry.
5. Arrive at searchable terms with an optional team filter.

Public content may be browsed before sign-in. Authentication is required for submission. A packaged, clearly dated public snapshot can support an immediate first-use/offline demonstration; it must never be presented as current remote content.

### B. Start editing from search

1. Search for a term or filter by team, class, and discussion status.
2. Open a term in a tab.
3. Read freely. On the first edit, create a local session automatically if none is active; offer a useful default name that can be changed inline.
4. Save draft changes automatically. Keep the session and tab state when the application closes.
5. Open more terms and continue editing in the same session.

### C. Start with a session

1. Open Sessions and choose “New session.”
2. Name it and optionally select a team or a list of terms to work through.
3. Search or open terms without leaving the session.
4. Close and reopen the application. Restore the active session, its tabs, and reading positions.

Selecting a work list is optional. Any eligible term can be added to the session later. Merely reading a term does not add it to the changed-term count.

### D. Submit a contribution

1. Choose “Review changes · N.”
2. Inspect every changed term, with a readable comparison and a source diff available.
3. Resolve blocking format errors; inspect content warnings.
4. Enter a short contribution title and description.
5. Choose “Submit for review.”
6. The backend creates the required fork/branch, one commit containing the session changes, and one PR. Display the resulting link and preserve it in the session.

### E. Continue a submitted contribution

1. Open the submitted session locally, or use “Resume from GitHub” to select an eligible PR authored by the signed-in user.
2. Check that the remote PR head still matches the last known revision.
3. Restore the proposed term content, make further edits, and autosave locally.
4. Review changes since the last submission and choose “Update submission.”
5. Append a commit to the same PR branch. Never create a second PR for that session.

### F. Work offline or recover

Cached terms and local drafts remain available without a connection. The interface says “Offline” and keeps local saving active. Submission waits until online. If an authorization or network operation fails, the draft remains intact and the user can retry or export the session.

## 5. Functional requirements

Priority P0 means release-blocking; P1 means important within the first usable release; Later means deliberately deferred.

| ID | Priority | Requirement | Acceptance outcome |
|---|---|---|---|
| AUTH-01 | P0 | Personal-token sign-in works without hosted infrastructure | A clean machine can submit with its own token; no shared secret is embedded |
| AUTH-02 | P0 | Remembered tokens stay in native credential storage | Token input clears after transfer; credentials never enter logs or exports |
| AUTH-03 | P1 | Token sign-in and sign-out | A user can supply a supported PAT; sign-out preserves local drafts |
| SEARCH-01 | P0 | Search term titles, synonyms, and definition text | Relevant terms open directly from results |
| SEARCH-02 | P0 | Filter by team, class, discussion status | Existing YAML tags work without data migration |
| SEARCH-03 | P1 | Changed-terms filter and keyboard quick search | An expert can return to the session’s work immediately |
| SESSION-01 | P0 | Create, name, reopen, and switch local sessions | Switching restores each session’s independent work |
| SESSION-02 | P0 | Local changes span many terms | One session can edit 30 terms without separate submissions |
| SESSION-03 | P0 | Each session links to at most one PR | Retrying or updating cannot silently create another PR |
| SESSION-04 | P0 | Import an eligible existing authored PR | Remote work can be resumed on another machine |
| SESSION-05 | P1 | Portable export/import | Another installation restores drafts and provenance without credentials |
| TAB-01 | P0 | Multiple term tabs with independent state | Editing one term cannot replace another tab’s content or history |
| TAB-02 | P0 | Closing a tab keeps saved changes | Reopening restores the draft; closing is not discarding |
| TAB-03 | P1 | Reorder, pin, overflow, reopen closed tab | Many tabs remain usable without wrapping the window chrome |
| EDIT-01 | P0 | Edit existing Markdown bodies visually | Headings, lists, links, tables, and scientific notation survive |
| EDIT-02 | P0 | Preserve alternative definitions and section structure | Accuracy and Resolution retain all definitions and empty sections |
| EDIT-03 | P0 | Preserve unedited source and metadata | Opening/closing a term produces no diff |
| EDIT-04 | P0 | Edit description and tags explicitly; title and required heading lines stay protected | Body edits do not automatically rewrite frontmatter |
| EDIT-07 | P0 | Create a new term with its complete skeleton visible immediately | Definition, Notes, Examples, Sources are present before typing content |
| EDIT-08 | P0 | Required structural headings are immutable during editing | Rich text and Rust section mutations both protect heading lines |
| EDIT-05 | P1 | Add an alternative definition | A deliberate action inserts the established section skeleton |
| EDIT-06 | P1 | Source view and safe unsupported-block handling | Unrecognized syntax is retained, never silently dropped |
| SAVE-01 | P0 | Automatic local saving | Saved status appears only after Rust acknowledges a durable write |
| SAVE-02 | P0 | Crash recovery | All acknowledged revisions survive forced termination |
| SAVE-03 | P0 | Explicit save errors and retry | A disk failure never leaves a false “Saved” state |
| SAVE-04 | P1 | Per-term revert and local revision recovery | An expert can undo an unwanted saved edit |
| GH-01 | P0 | Submit multiple files in one PR | Ten changed terms produce one PR, not ten |
| GH-02 | P0 | Update the same PR and prevent duplicate submissions | Network ambiguity is recovered using durable operation state |
| GH-03 | P0 | Support contributors without upstream write access | The app handles a personal fork when allowed by GitHub |
| GH-04 | P0 | Detect remote edits and conflicts | Concurrent work is not overwritten or force-pushed |
| REVIEW-01 | P0 | Review changes before sending | Changed terms and their content are inspectable together |
| REVIEW-02 | P1 | Show submission state and GitHub link | Users can distinguish local work from remotely submitted work |
| AGENT-01 | P1 | Documented session interchange and local command interface | An agent can prepare drafts without accessing tokens or submitting them |

## 6. Content rules and compatibility

The current corpus has 509 individual Markdown term files. The application keeps this format. It understands title, description, tags, numbered definitions, Notes, Examples, Sources, and alternative-definition separators.

Important invariants:

- File identity is the repository-relative path, not the display title. V1 does not rename files when a title changes.
- Unrecognized frontmatter fields and tags survive intact.
- Existing irregularities are diagnosed without automatic correction. Some current files have multiple class tags or unfinished sections.
- Changing a term does not automatically change its discussion status to approved.
- Session-local “checked” or “changed” indicators are independent of glossary status tags.
- Source citations are preserved. The editor does not insert the build system’s automatic glossary cross-links into source Markdown.
- Empty headings and duplicate/alternative definitions are valid draft content. Warnings can guide authors without preventing local saving.
- Blocks that cannot be edited safely remain available in source form. Publishing blocks only when the proposed result would be structurally invalid or destructive.

The exact source-preservation strategy and test corpus are specified in [Architecture](docs/ARCHITECTURE.md).

## 7. Interface concept

The main workspace has a narrow navigation rail for Terms and Sessions, a term list, and a document area with tabs. A compact top bar shows the active session and the review action. The editor receives most of the space.

The home state focuses search immediately and lists recent sessions beneath it. There is no analytics dashboard or large card grid. Term metadata is accessible from a small “Details” control; a review pane appears only when requested.

Use modern desktop conventions: quick search, persistent tabs, clear save feedback, keyboard navigation, predictable undo, and an inspectable change list. Use plain colours, system typography, thin borders, limited spacing tokens, and a single accent colour. See [UX specification](docs/UX.md) for wireframes and detailed behaviour.

## 8. Architecture decisions

1. **Rust owns durable data and GitHub operations.** Svelte owns presentation and transient interaction state.
2. **Direct GitHub APIs are the default.** This removes Git and GitHub CLI installation requirements for users. A sidecar remains an alternative adapter, not a parallel implementation to build speculatively.
3. **SQLite stores sessions and immutable source snapshots.** Portable JSON session files provide interchange, backup, and agent access.
4. **Draft saving and submission are separate.** Every local edit can be saved without creating commits, branches, or PRs.
5. **Markdown is the interchange/source format.** The rich editor’s internal document representation is not the only durable copy.
6. **Each submission is an immutable snapshot of selected revisions.** Continued editing during an upload cannot accidentally be marked submitted.
7. **No hosted backend, embedded AI dependency, or cloud draft database.** All online operations go directly to GitHub.

Detailed domain objects, state transitions, IPC, persistence, authentication, and concurrency rules are in [Architecture](docs/ARCHITECTURE.md). The [`gh` sidecar decision](docs/decisions/001-github-integration.md) weighs the proposed alternative explicitly.

## 9. Boundaries for the first release

Include the complete edit/save/review/submit/resume workflow. Include creation of term files with a visible skeleton. Defer deleting term files, repository-file renaming, real-time collaborative editing, embedded chat/AI generation, rich reviewer approval queues, automatic merges, multi-window editing, split-pane editors, and arbitrary Git hosting providers.

New terms and new alternative definitions are included. The required heading lines are rendered outside editable section bodies, and the Rust mutation boundary rejects structural heading changes.

A single active session is visible in a window. Multiple sessions can be stored, and the same term may have separate drafts in different sessions. One term has one editor buffer within a given session, even if the user tries to open it twice.

## 10. Delivery strategy and exit criteria

Implement vertical slices in this order:

1. Prove Markdown preservation and GitHub device authentication in small experiments.
2. Build the desktop shell, search, local sessions, persistence, and keyboard navigation.
3. Add WYSIWYG editing, term tabs, metadata, autosave, and crash recovery.
4. Add authenticated GitHub download, personal-fork submission, one-PR creation, and retry recovery.
5. Add authored-PR resumption, remote-change handling, review, export/import, and the agent interface.
6. Validate the entire expert workflow and package the supported platforms.

The release gate is a non-technical expert completing a multi-term session over two application launches, submitting it as one PR, then reopening it and updating that same PR. No terminal work, lost acknowledged changes, unrelated source changes, or forced Git operations are acceptable.

[Delivery and acceptance criteria](docs/DELIVERY.md) turns this into implementation milestones and an executable test plan. No timeline is asserted before the two technical experiments establish the uncertain work.

## 11. Dependencies to resolve during implementation

- Use personal access tokens as the primary route. GitHub App/OAuth deployment may be explored later, but no registration or client ID is required for release.
- Verify organization OAuth/SSO policies and public-fork behaviour with an ordinary contributor account, not only a maintainer account.
- Confirm the rich editor against the actual Markdown corpus before designing around its internal model.
- Select a native credential-storage library after testing all supported OS backends; secure-storage failure must not silently fall back to plaintext.
- Select the public code license and release identity before publishing binaries. Signed distribution depends on the maintainer’s platform signing credentials.

These are implementation/release dependencies. They do not prevent producing or reviewing this plan, and they do not require setting up hosted infrastructure.
