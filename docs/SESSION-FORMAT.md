# Portable session format, version 1

Export JSON contains `schemaVersion: 1`, `session`, and `catalogue`. It contains no credentials, OAuth device code, local filesystem paths, or native account secrets. The snapshot is included so a receiving installation can reproduce the original baseline offline.

`catalogue` contains repository identity, source commit SHA, branch, fetch timestamp, and a map of `docs/terms/<slug>.md` paths to original UTF-8 Markdown. GitHub repositories using the CEOS term-file format are supported. Repository identity is canonical `owner/repository`, independent of commit SHA. The default is `ceos-org/eo-glossary`; the current test repository is `do-me/eo-glossary`.

`session` contains:

- A stable UUID and name, repository and baseline SHA, and creation/update timestamps.
- `terms`: a map keyed by repository path. Each draft has `starting`, `source`, optional `submitted`, monotonic `revision`, and `isNew`.
- `workspace`: open/pinned tabs, active path, search/filter selections, changed-only flag, and per-term scroll offsets.
- Optional `submission`: PR number/URL, head repository/branch/revision, author account ID, and state.
- `operation`: null in portable exports. In-progress operations must be recovered or moved to a follow-up session first.

`starting` must match the pinned catalogue for an existing term. For a newly created file it is empty. `submitted` is the last successfully submitted frozen source; later edits are compared with it to find pending changes. Scientific status stays in the term's YAML tags.

Imports validate size (25 MiB), version, repository, UUID, baseline consistency, term paths, source parsing, and submission identity. Importing an already linked PR opens the existing local session; importing a duplicate unsubmitted session reports the duplication. Importing does not change the active catalogue snapshot.

Agents should use the CLI rather than editing SQLite rows. `edit` reads a JSON mutation from stdin and requires the expected current revision. Supported mutations:

```json
{"kind":"section","id":1,"markdown":"Revised definition, with a source citation."}
```

```json
{"kind":"metadata","description":"Short description.","tags":["core","to be discussed","WGCV"]}
```

```json
{"kind":"add_definition"}
```

Obtain section IDs and revision from `open`. The title and structural heading lines cannot be edited through these mutations. New terms are created using `create-term SESSION_ID "Term name" --team WGCV`. The CLI never accesses the system credential store and cannot submit remotely.

A session export includes the complete pinned public catalogue as well as drafts. Treat the file according to the sensitivity of your unpublished edits. SQLite backup requires a consistent backup or copying its database/WAL/SHM together while the application is stopped; portable export is the supported interchange workflow.

Local SQLite schema version 2 migrates version 1 transactionally. Snapshot identity is now `(repository, sha)`, and PR identity is `(repository, number)`. Switching repository preferences does not retarget a session or its interrupted submission. The operation journal records its target repository and base branch; legacy operations default to the original CEOS repository and `main`.
