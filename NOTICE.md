# Source and dependency notices

`resources/catalogue.json` embeds the public CEOS EO glossary's 509 term files for immediate offline use. Source: https://github.com/ceos-org/eo-glossary/tree/62edc6225cee8219b217f5cbacda2830ab48dc98/docs/terms, fetched 26 October 2026. Attribution and citations inside term files are preserved. These glossary texts are not relicensed by this editor's application-code license. The upstream repository did not expose a root license through GitHub's repository metadata when this local build was prepared; establish the glossary redistribution terms before publishing binaries containing the snapshot.

The application uses open-source dependencies including Tauri, Svelte, Tailwind CSS, Milkdown/ProseMirror, Lucide, SQLite/rusqlite, reqwest, and their transitive dependencies. Locked dependency graphs are in `Cargo.lock` and `package-lock.json`. Retain applicable dependency license notices in public distributions.
