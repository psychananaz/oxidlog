# Internal API

OxidLog is a binary crate. These modules are internal APIs; there is no public Rust library target. Command handlers coordinate operations, while parsing, selection, formatting, and file access have separate entry points.

| Module | Responsibility |
| --- | --- |
| `cli` | Parse arguments, load the files each command needs, and attach command context to errors. |
| `cli::input` | Prompt for edit input and report input/output failures. |
| `commands` | Coordinate a command's validation, journal operations, persistence, and output. |
| `content` | Parse body text and normalize/deduplicate tags for add and edit. |
| `storage::journal` | Own entries and enforce identity and mutation rules without performing file I/O. |
| `query` | Apply shared date/tag filters and select borrowed entries. |
| `matching` | Prepare text queries and locate matches in the original body. |
| `presentation` | Format entries and write results to an `io::Write` destination. |
| `storage` | Resolve paths, load/save data, initialize files, and manage backups. |
| `error` | Represent failure causes and preserve source errors. |

## Search and view

`query::select(&[Entry], &EntryFilter)` returns `Vec<&Entry>`.

`query::search(&[Entry], &SearchQuery)` returns `Vec<SearchMatch<'_>>`. Each result borrows an entry and owns a list of byte ranges identifying matches in that entry's original body. Search neither prints nor formats entries. Once query construction has validated the date range, selection is infallible; no results is a normal empty vector.

The search command prepares a query once, selects results, and passes them to `presentation::write_search_results`. The view command uses the same date/tag filter and calls `presentation::write_entries` or `write_detail`. Formatting reads entry data and uses small, copyable `DisplayOptions`; it does not own or clone the journal configuration.

Case-sensitive text matching borrows the body. Case-insensitive matching lowercases it and maps matches back to the original character boundaries. This uses Unicode lowercasing, not full Unicode case folding or normalization. Fuzzy matching means characters appearing in order, not edit-distance ranking. Only body text is searched and highlighted, not dates or the separate tag header. Color follows the `colored` crate's terminal/environment policy.

## Journal mutations

`Journal` owns its path and entries. `path()` returns `&Path`; `entries()` returns `&[Entry]`. Callers use `get_entry(id)` for read access, `add_entry(body, tags)` to allocate an ID, and `edit_entry(id, content, tags)` to change editable fields while retaining identity and timestamps.

IDs start at zero. A new ID is one greater than the highest currently stored ID; adding fails on integer exhaustion. Loading rejects duplicate IDs rather than silently renumbering existing data. The JSON array format is unchanged, so there is no persisted historical ID counter: deleting the highest IDs or clearing the journal can allow those deleted IDs to be reused.

Removal selects and validates its full set before mutation, then saves once. Explicit missing IDs are errors. ID ranges are inclusive (`0..2` includes 0, 1, and 2) and select existing entries, allowing gaps. Multiple selectors form a union without duplicate removals.

## Storage and errors

Configuration loading has no write side effects. The CLI parses arguments before loading files, so help and initialization remain available with a malformed config. Backup restoration can operate when the current journal is missing or corrupt, but validates the backup before replacement.

Journal/config replacements use unique, exclusively created sibling temporary files, synchronize their contents, and rename them into place. Backups are retained before replacing existing data. Initialization saves configuration before resetting the journal and retains the previous journal as a backup. These are per-file operations, not a transaction spanning all files. There is no interprocess locking; simultaneous writers can still overwrite each other's changes.

`AppError` identifies causes such as `EntryNotFound`, `DuplicateId`, `EmptyBody`, invalid date ranges, malformed files, and I/O failures. File errors include an operation and path while retaining their original error. The CLI attaches the command name. Presentation and prompts return I/O failures; a broken output pipe exits quietly. There are no placeholder `SearchError` or `BackupError` variants.

## Verification

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Unit tests live beside their modules. CLI integration tests run child processes with separate temporary `XLOG_HOME` directories. Initialization's file operations are unit-tested, including failure preservation; its successful terminal wizard and external file-opening programs are not automated.
