# EO Glossary Editor

A desktop editor for [the CEOS Earth observation glossary](https://github.com/ceos-org/eo-glossary), built with Tauri 2, Rust, Svelte 5, TypeScript, and Tailwind CSS 4. It implements the expert workflow discussed in [issue #13](https://github.com/ceos-org/eo-glossary/issues/13).

Search terms, filter by review team or discussion status, edit several terms in tabs, and resume your work later. One editing session produces one GitHub pull request; subsequent submissions update that PR.

<img width="1645" height="1095" alt="image" src="https://github.com/user-attachments/assets/800e44e6-0def-43fd-bc05-71df98a05cf5" />
<img width="1645" height="1095" alt="image" src="https://github.com/user-attachments/assets/ae7001f5-c7d4-45d3-b1af-5a16f302f830" />
<img width="1645" height="1095" alt="image" src="https://github.com/user-attachments/assets/b2602942-5d34-4db5-b32d-066d4638195b" />

## Desktop app download

Download [v0.1.1](https://github.com/do-me/eo-glossary-editor/releases/tag/v0.1.1) for your computer. macOS packages are ad-hoc signed without notarization; Windows installers are unsigned. Use the commands below only for the release downloaded from this repository that you trust. Commands assume the installer is in your Downloads folder.

- **macOS** — [Apple Silicon DMG](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_macos_aarch64.dmg) · [Intel DMG](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_macos_x64.dmg). Open the DMG and drag **EO Glossary Editor.app** into **Applications**. If macOS blocks opening it, remove this app's quarantine attribute once, then open it:

  ```sh
  xattr -dr com.apple.quarantine "/Applications/EO Glossary Editor.app"
  open "/Applications/EO Glossary Editor.app"
  ```

  You can also use **System Settings → Privacy & Security → Open Anyway** after a failed launch attempt.

- **Windows x64 (setup)** — [Download EXE](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_windows_x64-setup.exe). Double-click, or use PowerShell:

  ```powershell
  Unblock-File -LiteralPath "$HOME\Downloads\EO-Glossary-Editor_0.1.1_windows_x64-setup.exe"
  Start-Process -FilePath "$HOME\Downloads\EO-Glossary-Editor_0.1.1_windows_x64-setup.exe" -Wait
  ```

- **Windows x64 (MSI)** — [Download MSI](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_windows_x64.msi). Double-click, or use PowerShell:

  ```powershell
  Unblock-File -LiteralPath "$HOME\Downloads\EO-Glossary-Editor_0.1.1_windows_x64.msi"
  msiexec.exe /i "$HOME\Downloads\EO-Glossary-Editor_0.1.1_windows_x64.msi"
  ```

  Windows may still show SmartScreen: choose **More info → Run anyway** if offered and you trust the download. Unblock-File removes the file's download mark; it does not override Smart App Control or organizational policies.

- **Linux x64 (AppImage)** — [Download AppImage](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_linux_amd64.AppImage). Make the downloaded file executable, then run it:

  ```sh
  cd "$HOME/Downloads"
  chmod u+x ./EO-Glossary-Editor_0.1.1_linux_amd64.AppImage
  ./EO-Glossary-Editor_0.1.1_linux_amd64.AppImage
  ```

  If the system has no FUSE runtime, try `./EO-Glossary-Editor_0.1.1_linux_amd64.AppImage --appimage-extract-and-run`.

- **Linux x64 (Debian / Ubuntu)** — [Download DEB](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_linux_amd64.deb). Install with dependencies, then launch from your application menu or terminal:

  ```sh
  cd "$HOME/Downloads"
  sudo apt install ./EO-Glossary-Editor_0.1.1_linux_amd64.deb
  eo-glossary-editor
  ```

- **Linux x64 (Fedora / RHEL)** — [Download RPM](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/EO-Glossary-Editor_0.1.1_linux_x86_64.rpm). Install with dependencies, then launch from your application menu or terminal:

  ```sh
  cd "$HOME/Downloads"
  sudo dnf install ./EO-Glossary-Editor_0.1.1_linux_x86_64.rpm
  eo-glossary-editor
  ```

[SHA-256 checksums](https://github.com/do-me/eo-glossary-editor/releases/download/v0.1.1/SHA256SUMS.txt) are included with the release. Interactive platform testing has been performed on macOS; all installers are built in CI.

Start with **Settings → Personal access token → Sign in with token**. See [token setup](docs/GITHUB-SETUP.md). The default glossary target is `ceos-org/eo-glossary`; the editor source is `do-me/eo-glossary-editor`.

## Run locally

Prerequisites for **development**: Node 24 LTS, npm, stable Rust, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/). End users of a packaged app do not need these tools, Git, or GitHub CLI.

```sh
npm ci
npm run desktop
```

Build a macOS app:

```sh
npm run tauri build -- --bundles app
open "target/release/bundle/macos/EO Glossary Editor.app"
```

A macOS build produces `target/release/bundle/macos/EO Glossary Editor.app` (about 14 MB). Native macOS use and all platform installer builds are checked. Interactive Windows/Linux trials and certificate-signed distribution remain future validation.

## Editing

- **Terms:** search titles and source text; combine team, class, and discussion-status filters. Team selections are ORed; different filter groups are ANDed.
- **Tabs:** open multiple terms, pin or move tabs using their menu, and close tabs without discarding drafts. Only the active document mounts rich editors.
- **New term:** see the complete skeleton immediately, enter a title, optionally assign teams, and create the file. Definition, Notes, Examples, and Sources are ready to fill in.
- **Protected structure:** the title and required section headings live outside editable content. Rust also rejects structural headings in section edits, including source edits. Add alternative definitions with the dedicated action.
- **Autosave:** `Saved locally` appears only after a durable SQLite acknowledgement. Errors remain visible and pending edits can be retried. Tab/session transitions and window closure flush pending saves.
- **Source:** unsupported HTML, math, and reference syntax uses section-level Markdown editing. The complete source is also available read-only. Untouched sections and unedited frontmatter remain byte-for-byte intact; a rich-edited section uses Milkdown’s Markdown formatting.
- **Sessions:** create or rename sessions, switch between independent drafts, export/import portable JSON, and resume an open PR authored by your GitHub account.
- **Repository settings:** choose a GitHub repository URL under Account and settings. Switching downloads its default branch and opens a fresh session; existing sessions keep their original repository. The session header and review screen show the destination. Snapshots and PR identities are scoped by repository, so a fork sharing a commit SHA or PR number remains separate.
- **After submission:** one durable PR summary shows the PR state and submitted term count. Full contribution and Not submitted separate the complete proposal from newer local edits. Successful submission brings the summary into view. Merged/closed contributions remain reviewable and offer a follow-up session.
- **Review:** read rendered terms with word-level addition/removal highlights. Switch between Changes, a clean Preview, and Side by side. The source diff remains collapsed and available for precise inspection. Review-team, class, and empty-section notes guide authors without automatically changing scientific status.

**Zoom and recovery:** the bottom zoom control and `⌘/Ctrl +`, `⌘/Ctrl −`, and `⌘/Ctrl 0` support 75–200% zoom, saved locally. Normal quit/reopen restores native window size/position/fullscreen, active session, tabs, search/filters, page, document panels, scroll positions, and contribution text. Rich-editor caret and undo history are transient. Selecting a session deliberately returns to its editing page.

Keyboard shortcuts: `⌘/Ctrl K` search, `⌘/Ctrl S` flush saves, `⌘/Ctrl W` close term tab, `⌘/Ctrl Shift T` reopen the last closed tab. Rich editor undo/redo uses standard platform shortcuts; its undo history lasts for the mounted editor. Saved content persists across tab switches and restarts.

## GitHub authentication

**Personal access tokens are the primary sign-in method.** No app registration, hosted service, Git installation, or CLI sidecar is needed.

1. Open [GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic) → Generate new token](https://github.com/settings/tokens/new).
2. Give it a name and expiration, and select **public_repo** for the public-glossary workflow.
3. In the editor's **Settings**, paste it into **Personal access token** and click **Sign in with token**.

Remembered tokens stay in the system credential store. Uncheck Remember me to use the token for this launch only. The input clears immediately when submitted; session files and exports exclude credentials. Fine-grained token permissions and organization/SSO restrictions are covered in [token setup](docs/GITHUB-SETUP.md).

GitHub App / OAuth device authorization is an optional future direction, with existing experimental support retained under **Advanced: optional app sign-in**. It is not required for releases or normal use, and there is no commitment to roll it out. See [the authentication decision](docs/decisions/003-personal-access-tokens.md).

## Submission and recovery

Rust freezes the pending files into a durable operation journal, creates one tree/commit, advances a stable session branch without force-pushing, then creates or reuses its PR. Contributors without upstream write access use a personal fork. Retries look up the stable branch before creating a PR again.

Updates check author identity, PR state, and the current remote head. Non-overlapping remote edits merge automatically. Overlapping edits, removed files, and unexpected branch changes stop submission while retaining local work; conflicts on the PR branch itself still require reconciliation on GitHub. Main-versus-draft conflicts have in-app version choices. If a previously submitted term is deleted on main, removing it from an existing PR must be done on GitHub; keeping it locally can propose its restoration. The app does not silently overwrite them. `Start follow-up` creates a fresh session for remaining local work while preserving the original session and its interrupted operation. Finish/recover an in-progress operation before exporting that original session.

An imported PR must be open, authored by the signed-in account, and contain only added/modified term files. Rename/delete or unrelated file changes are reported explicitly.

The maintainer has successfully tested personal-token authentication, commits, and a PR in the sandbox glossary. Device authorization and expiring-token refresh still need live validation. Deterministic HTTP tests cover the fork path, multi-file commit, same-PR updates, response-loss recovery, and remote-change refusal. Keep further integration trials in a dedicated sandbox repository.

## Local data and agent interface

Native data lives under the OS app-data directory for `org.ceos.glossary-editor`:

- macOS: `~/Library/Application Support/org.ceos.glossary-editor/sessions.sqlite3`
- Windows: the Tauri app-data location under the user's roaming profile.
- Linux: the Tauri app-data location under `$XDG_DATA_HOME` (normally `~/.local/share`).

The app includes 509 term files from main commit `62edc6225cee8219b217f5cbacda2830ab48dc98`, fetched 26 October 2026. Refresh downloads a new consistent snapshot. At each opening, the app fetches the current default branch (main for CEOS). Clean updates and non-overlapping local edits reconcile automatically. Conflicting terms retain both versions and show rendered comparisons plus precise source diffs. Choose Use GitHub version to replace the draft, or Keep my version to retain it against the new baseline. Publication waits until each conflict has a choice. Earlier draft revisions are retained locally; export provides a portable backup. Cached editing remains available offline.

```sh
cargo run -p eo-editor-cli -- --database /path/to/sessions.sqlite3 sessions
cargo run -p eo-editor-cli -- --database /path/to/sessions.sqlite3 new "WGCV review"
cargo run -p eo-editor-cli -- --database /path/to/sessions.sqlite3 open SESSION_ID docs/terms/accuracy.md
printf '%s' '{"kind":"section","id":1,"markdown":"A more precise definition."}' | \
  cargo run -p eo-editor-cli -- --database /path/to/sessions.sqlite3 edit SESSION_ID docs/terms/accuracy.md --revision 0
cargo run -p eo-editor-cli -- --database /path/to/sessions.sqlite3 export SESSION_ID > review.json
```

The CLI has no authentication or publication commands. Its revision checks prevent stale writes from replacing newer app edits. See [the session format](docs/SESSION-FORMAT.md).

## Verify

```sh
npm run check
npm run lint
npm run format:check
npm test
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p editor-core
npx playwright install chromium
npm run test:e2e
npm audit
```

Browser workflow tests run the real Rust commands against an isolated SQLite database, through a loopback bridge compiled only with the CLI's `test-server` feature. The development frontend requires explicit endpoint/token environment variables. Production builds cannot use this bridge and the desktop app contains no HTTP server.

[Validation and limits](docs/VALIDATION.md) records what has actually been tested. [Plan](PLAN.md), [UX](docs/UX.md), [architecture](docs/ARCHITECTURE.md), and [GitHub integration decision](docs/decisions/001-github-integration.md) document the product intent and implementation.

Source: [do-me/eo-glossary-editor](https://github.com/do-me/eo-glossary-editor). Desktop packages are published through the release workflow; the production glossary is not used for automated test PRs. Authored application code is MIT licensed; see [LICENSE](LICENSE). Bundled glossary source retains its upstream provenance; see [NOTICE](NOTICE.md) before redistributing it.
