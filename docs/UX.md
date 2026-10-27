# Interface and interaction specification

## 1. Design direction

Build a quiet desktop workspace for reading, comparing, and editing scientific language. The interface should make three actions immediately discoverable: **find a term**, **resume a session**, and **edit a term**.

Use established desktop interaction patterns: searchable lists, persistent document tabs, a readable document canvas, local-save feedback, and a single review action for the whole contribution. “State of the art” here means responsive, accessible, recoverable, and predictable. It does not require an elaborate visual identity.

The main conceptual objects are Term, Session, Local changes, and Submission. Branches, commits, Git object identifiers, and authentication scopes are available in diagnostics, not in the primary editing flow.

## 2. Application structure

| Surface | Purpose | Primary action |
|---|---|---|
| Start | Find a term or reopen recent work | Search terms |
| Terms workspace | Search, filter, read, and edit | Open a term |
| Sessions | Create, resume, export, and inspect contributions | Resume / New session |
| Term editor | Work on content across tabs | Edit; saves automatically |
| Review changes | Inspect the full session and send it | Submit for review / Update submission |
| Settings | Account, appearance, local data, diagnostics | Sign in with a token |

Keep Terms and Sessions available in a narrow left rail. Settings and the account state sit at its bottom. The rail uses an icon and a visible text label or an equally accessible expanded form; navigation must not depend on guessing icons.

## 3. Main workspace wireframe

At a typical 1280 × 800 window:

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ EO Glossary Editor       WGCV terminology review ▾              Review changes · 4       │
├──────────┬────────────────────────────┬──────────────────────────────────────────────────┤
│ Terms    │ Search terms…        ⌘K    │ Accuracy × │ Precision × │ Uncertainty × │   ▾   │
│ Sessions │                            ├──────────────────────────────────────────────────┤
│          │ Team: WGCV            ▾    │ Accuracy                              Details   │
│          │ Status: To be discussed ▾  │                                                  │
│          │ Class: All            ▾    │ B  I   Link   List   Table           Outline ▾   │
│          │ Changed only          ☐    │                                                  │
│          │                            │  1 Definition                                    │
│          │  168 terms                 │                                                  │
│          │  Accuracy              •   │  Closeness of agreement between a measured       │
│          │  Calibration               │  quantity value and a true quantity value…      │
│          │  Precision             •   │                                                  │
│          │  Reference                 │  Notes                                           │
│          │  Uncertainty           •   │  • In essence, Accuracy reflects…               │
│          │  Validation                │                                                  │
│          │                            │  Sources                                         │
│          │                            │  • BS ISO 5725-1                                  │
│          │                            │                                                  │
│ Settings │                            │  2 Definition                                    │
├──────────┴────────────────────────────┴──────────────────────────────────────────────────┤
│ Saved locally · 14:32                 4 changed terms                    GitHub connected │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

The numbers and content are illustrative. The term list uses a quiet changed indicator with an accessible label. It does not use the same indicator to mean “unsaved,” “approved,” and “selected.”

Initial dimensions: navigation rail 76–96 px, term sidebar 260–300 px, document canvas filling the rest. Sidebars are collapsible. Avoid a permanent right-hand inspector; Details and Review appear when requested.

## 4. Start and first-use screens

The first screen presents the repository identity, one large search input, a short recent-session list, and authentication status. There is no onboarding carousel.

Before the first download, show:

- “EO Glossary · ceos-org/eo-glossary.”
- “Sign in with a token.”
- “Browse public terms” where public downloading is available.
- A concise explanation: drafts stay on this computer until the contributor submits them.

Settings displays the personal-token password field and token-creation link immediately. Show rejection errors beside the form, clear the input after each attempt, and expose Remember me for the OS credential store. On success, show the actual GitHub account. Existing app/device authorization remains collapsed under Advanced: optional app sign-in as a possible future experiment, without being an onboarding requirement.

After sign-out, show locally saved sessions and cached terms as before. Disable submission until the correct account reconnects.

## 5. Search and discovery

### Search behaviour

- Search is available from the start screen, the Terms sidebar, and the quick-search dialog.
- Rank exact and prefix title matches ahead of synonyms and definition text. Show a short matching excerpt when the match is in the body.
- Use deterministic local search. No external search service or embedding service is required.
- Match case-insensitively; normalize accents and underscore/space differences for matching while retaining original display text.
- Arrow keys navigate results; Enter opens the highlighted term in a persistent tab. Escape closes quick search without changing the current editor.
- Opening a result that is already open in the current session focuses that tab.
- Results should update without blocking typing or moving focus away from the search input.

### Filters

Offer Team, Discussion status, and Term class. Show selected values as compact text controls with a clear-all action. Within a filter group, multiple selections mean OR. Different groups combine with AND. Thus `(WGCV OR LSI-VC) AND to be discussed` has an unambiguous meaning.

Keep class, status, and team concepts separate in the UI even though the source stores them in one YAML tag list. Unknown/source-history tags remain visible in Details and are preserved.

Filters apply to the list; they do not close open tabs. Editing a tag may remove the term from the filtered results, but its tab stays open. A session’s changed-terms view remains available regardless of current filters.

Remember filters per session. Offer a neutral “My teams” preference for convenience; this is not an authorization rule or a restriction on which terms may be edited.

### Reading context

Sources are readable links. Opening a source sends the URL to the system browser. Existing glossary references can open another term tab when resolvable, without inserting cross-link markup into the stored file. A lightweight outline popover jumps between definitions and their sections.

## 6. Session management

Use a plain list or table rather than a card dashboard:

```text
Sessions                                             New session
Search sessions…                            Resume from GitHub…

Name                        Local changes   Submission       Updated
WGCV terminology review     4 terms         Local only       Today
Land imaging terminology    2 terms         PR #24 · Open     Yesterday
Data access review          —               PR #19 · Merged   20 Oct
```

“Local changes” means differences not yet sent in the current session. The detail view also reports the total proposed terms in the PR. This distinction matters after a submission has been updated several times.

Session actions: open, rename, export, archive locally, and inspect the submission on GitHub. A separate destructive action removes local session data; it explains what would be removed and does not close or delete a GitHub PR.

Creation asks only for a name. Team filters, a starting term list, and a session note are optional. From the first edit, automatic session creation uses a name such as “Terminology review · 26 Oct”; the user can rename it without a modal.

Switching sessions flushes pending writes and restores the target’s tab order, active term, scroll position, filters, and search state. Sessions never share mutable draft buffers.

“Resume from GitHub” lists the signed-in user’s eligible PRs targeting the configured repository. It also accepts a PR URL. Show title, author, status, and changed-term count before importing. Existing locally linked PRs open their existing session instead of creating a duplicate.

## 7. Tabs

- One tab per `(session ID, term path)`. Opening a term twice focuses the existing tab.
- Each tab retains selection, scroll position, undo/redo history during the current process, and its independently saved draft.
- Tab titles use the term title; disambiguate identical titles with the file identifier in a tooltip/menu.
- Support drag reordering, explicit pinning, and an overflow menu. Keep the strip to one line.
- Close buttons are visible on the active tab and on hover/focus elsewhere; keyboard closing is always available.
- Closing a tab flushes pending saving. It leaves the term’s saved draft in the session.
- “Discard changes” is an explicit per-term action in Review or the term menu, never an effect of closing a tab.
- Restore the workspace after restart. Restoring a closed tab restores its draft, not just the base file.
- Many tabs do not require many mounted rich-text editors. Keep lightweight document state for inactive tabs and mount the active editor; preserve undo history where practical during tab transitions.
- A separate small save-state indicator can show pending/failed writes. A changed-from-base marker continues to exist after successful local saving.

V1 uses one active session per window. Tab drag-out, split editors, and multiple windows are deferred to avoid concurrent editing ambiguity.

## 8. Term editing

### Document surface

The title, numbered definitions, Notes, Examples, and Sources read like a formatted document. Display the rich-text body at a comfortable line length, with generous vertical separation between sections. Keep all alternative definitions accessible within the same document.

Use a small toolbar for the operations needed by this glossary: bold, italic, lists, links, tables, undo/redo, and an overflow menu. Common formatting also works through keyboard shortcuts and an optional selection toolbar. Avoid a large word-processor ribbon or a slash-command system that must be learned before basic editing.

The section structure is recognizable rather than hidden inside nested form panels. An explicit “Add alternative definition” action inserts the correct numbered block and sections. Arbitrary unsupported blocks remain visible as protected source blocks or in Source view.

### Details

Details opens a compact inspector with:

- Title.
- Description.
- Discussion status.
- Term class.
- Team tags, with search and multi-select.
- Additional existing tags in a quieter group.
- File identifier, source snapshot date, and diagnostics.

Changing a title does not rename a repository file. Title/H1 synchronization is an explicit, previewable operation on recognized heading structure. Editing a definition does not regenerate the description automatically. All existing unknown metadata is retained.

### Content guidance

Offer concise, non-blocking diagnostics for a missing source, missing definition text, or a definition that starts with “Term is…”. These are authoring aids; automated heuristics cannot certify scientific correctness, detect every circular definition, or approve a term.

Always allow incomplete work to save locally. Submission distinguishes structural errors that could corrupt the file from content warnings that belong in expert review. Existing baseline warnings are identified separately from newly introduced problems.

### Source mode

Keep “View source” in the editor menu and use the same draft as rich-text mode. Switching modes first flushes changes and checks that parsing is safe. Never rebuild the document just because a user toggles a view. Unsupported content can be retained and edited deliberately in source mode.

## 9. Saving and recovery feedback

Use precise, stable labels:

| Label | Meaning |
|---|---|
| Saving locally… | The most recent edit is not yet acknowledged by Rust |
| Saved locally · time | The current revision has been committed to local durable storage |
| Couldn’t save locally · Retry | Saving failed; edits remain available in memory for recovery/export |
| Changes not submitted | Local draft is newer than the last submitted revision |
| Submitted · PR #N | The displayed revision was sent successfully |
| Offline | Remote operations are unavailable; local saving still works |

Use a 500 ms idle debounce initially, with a maximum two-second persistence interval while continuously typing. Flush on tab/session changes, application blur where possible, and orderly window close. The exact guarantee is survival of every acknowledged revision; no application can promise to persist keystrokes still in flight at forced process termination.

Failure messages sit near the save state and affected action. Do not interrupt normal typing with repeated notifications. Provide a recovery/export path when storage fails. Screen-reader announcements are throttled and use polite live regions.

## 10. Reviewing and submitting a session

```text
Review changes                                  Back to editing

4 changed terms                                 Local revisions saved
  Accuracy          Definition, Notes
  Precision         Sources
  Uncertainty       Definition
  Validation        Team tags

Accuracy                                      Readable | Source
  Previous                                      Proposed
  …                                              …

Contribution title
[Clarify accuracy, precision, and uncertainty                 ]

Description
[Short explanation of the intended scientific changes…       ]

2 content warnings · inspect
                                         Submit for review
```

Review is a workspace mode, not a small modal. For narrow windows, stack Previous and Proposed or use a unified word diff. Metadata changes are listed separately from prose changes.

The default action sends all currently changed terms in the active session. V1 does not introduce hidden per-file staging. To send unrelated work separately, use a different session. Before submission, any term can be deliberately reverted or moved to a separate session through a clearly named action in a later enhancement.

After the first submission, use “Update submission,” show the PR number, and default the review comparison to changes since the last submission. Offer “All proposed changes” for the complete base-to-draft view.

Freeze a revision snapshot for upload. If the user continues editing, indicate that newer local changes remain after the uploaded version. A save during upload must never be silently treated as submitted.

Merged submissions become historical/read-only sessions; “Start a follow-up session” loads current upstream content. A closed, unmerged PR is shown explicitly; it is not reopened or replaced automatically.

## 11. Conflicts and exceptional states

When a remote term or session branch has changed, preserve the local draft and explain which term needs attention. Compare **starting version**, **your changes**, and **latest version**. Offer automatic combination only for demonstrably non-overlapping changes; overlapping changes require a readable choice or manual edit.

Do not expose raw conflict markers as the primary interface. “Keep mine” means create a reviewed combined revision from the latest version, not force-push over remote work.

Missing/deleted remote files, revoked access, changed account, unavailable credential store, and a session imported from another repository receive specific messages. Local work remains accessible throughout.

## 12. Visual system

| Element | Initial specification |
|---|---|
| Typeface | System UI stack; no remote font dependency |
| Editor text | 16 px, approximately 1.6 line height, 68–76 character measure |
| Chrome text | 13–14 px, readable rather than compressed |
| Background | White/light neutral; system dark mode with equivalent hierarchy |
| Accent | One blue accent for primary actions, active navigation, and focus |
| Borders | 1 px neutral separators; avoid nested boxed containers |
| Radius | Approximately 6 px for inputs/buttons, small consistent scale |
| Spacing | 4, 8, 12, 16, 24, 32 px tokens |
| Elevation | Menus/dialogs only; no decorative shadows or gradients |
| Tags | Quiet text/outlined chips, grouped by meaning; no rainbow taxonomy |
| Icons | Small consistent set, with text for consequential actions |
| Motion | Minimal; respect reduced-motion preferences |

Use Tailwind for layout and ordinary control styling. Keep a small shared token stylesheet and one scoped prose/editor stylesheet. Avoid importing a full themed component suite or building a parallel custom CSS framework.

## 13. Keyboard and accessibility

| Shortcut | Action |
|---|---|
| Cmd/Ctrl+K | Search terms |
| Cmd/Ctrl+Shift+P | Small command menu for sessions and app actions |
| Cmd/Ctrl+W | Close active tab, with save flush |
| Cmd/Ctrl+Shift+T | Reopen last closed tab |
| Ctrl+Tab / Ctrl+Shift+Tab | Cycle document tabs |
| Cmd/Ctrl+S | Flush pending local saving; never submit |
| Cmd/Ctrl+Z | Undo active document edit |
| Cmd/Ctrl+Shift+Z | Redo active document edit |
| Cmd/Ctrl+, | Settings |
| Escape | Close the current transient surface |

Ensure native platform conventions where shortcuts differ. Do not intercept ordinary text-editing shortcuts without an explicit reason.

Provide complete keyboard access, visible focus, semantic buttons, properly implemented tab roles, named inputs, sufficient contrast, text scaling, and screen-reader labels. Provide keyboard alternatives to dragging/reordering. Selecting a search result should put focus in a predictable reading location; background list updates should never steal focus.

At small widths, collapse the navigation/list rather than shrinking document text. The target remains a desktop application; mobile layouts are not a release requirement.

## 14. Design validation

Before implementing all screens, validate three tasks against the concept:

1. Find the terms assigned to a team and open three tabs.
2. Change two terms, close a tab, restart, and explain where the work is saved.
3. Submit a session and later identify how to update the same submission.

If an expert confuses “saved locally” with “submitted,” or closes a tab expecting to discard changes, revise the labels and interaction before adding features. The prototype stage should test these distinctions with representative experts, not only developers.


## Confirmed implementation refinements

New-term creation is part of this build: the creation dialog shows the full skeleton immediately, before the name is entered. Creation opens that same structure in a document tab. The title and required Definition, Notes, Examples, and Sources headings are rendered outside editable bodies. Rich and source editing change section content only; the Rust command boundary enforces this rule as well. Add alternative definition inserts its own protected skeleton. See `VALIDATION.md` for measured implementation coverage.


## Implemented recovery controls

The footer exposes a zoom slider, increment/decrement, and reset from 75% to 200%. Ctrl/Cmd plus, minus, and zero use the same persisted preference. On reopening, restore window geometry and the saved workspace, including panels, filters, source views, reading offsets, and unfinished contribution text. Selecting a different session deliberately opens its editing page.

Main is checked in the background at startup. Apply independent updates after a brief save-and-reload step; keep the workspace usable while the network request runs. For overlaps, show a persistent Review differences notice. Each term presents Latest GitHub and Your local version as rendered pages, with optional exact diffs against the common ancestor. Use GitHub version replaces only the reviewed term, retaining its prior revision locally; Keep my version retains the draft against the latest baseline. Unresolved conflicts prevent publication. Export a backup remains available. Frozen uploads recover before applying main updates.


## Submitted contribution review

Use a durable PR summary derived from the session link, rather than a transient success banner. The card states Open/Merged/Closed, the submitted term count, and whether saved local edits remain outside the PR. Its primary action opens the PR; the secondary action continues editing or starts a follow-up. Default to the full contribution after successful submission; distinguish it from Not submitted with an accessible pressed-state scope control. Retain comparison modes, export, source diffs, and upload recovery. After success, scroll to and focus the summary heading so the outcome is immediately visible.
