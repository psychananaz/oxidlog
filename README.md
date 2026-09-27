# [WIP] OxidLog - A Personal Knowledge Manager

This is a powerful tool that allows you to take notes, todos, or even journal!
With many different features you can make this tool what you need it to be.
The only limit is.. text!!

## Quick Start

```bash
# Install OxidLog
cargo install oxidlog

# Initialize your journal
xlog init

# Create your first entry
xlog add "Starting my journey with OxidLog"
```

## Key Features

### Smart Tagging
Organize entries with hashtags
```bash
xlog add "Meeting notes #work #important"
```

### Easy Retrieval
View all entries with a simple command
```bash
xlog view
```

### Custom Filters
Filter entries by tags
```bash
xlog view --tags "work"
```
Or by specifying a timeframe
```bash
xlog view --from "2021-01-01" --to "2021-12-31"
```

### Search Entries
Find specific entries using keywords
```bash
xlog search "meeting" --tags "important"
```

## Commands

### `xlog init`
Initialize a new journal or reconfigure an existing one.

### `xlog add "content"`
Add a new entry to your journal with the specified content.

### `xlog remove [id]`
Remove an entry from your journal by its ID.

### `xlog view`
View all journal entries.

### `xlog edit [id]`
Edit an existing journal entry by its ID.

### `xlog search "query"`
Search through journal entries using a query.

### `xlog export [json|csv|plain]`
Export journal entries to various formats.

### `xlog backup [create|restore]`
Create or restore a backup of your journal.

## Data and Config Location

The data and config files are located in the `.oxidlog` directory in your home folder. The config file is named `config.toml` and the journal data is stored in `journal.json`.

Set `XLOG_HOME` to override the directory containing `config.toml` and `journal.json`.
Without this override, debug builds use `.oxidlog` in the project directory and release builds use `.oxidlog` in your home directory.

## Testing

Run `cargo test` to run all tests. Unit tests live in `#[cfg(test)] mod tests` beside the code in `src/`. This project is a binary crate, so the integration tests in `tests/cli_tests.rs` launch `xlog` and check its output and saved files instead of importing internal modules.

Each CLI test uses a temporary `XLOG_HOME`. The tests create journal fixtures directly because `init` uses an interactive terminal UI. They cover its failure without a terminal, but do not automate the successful interactive wizard.

## Learn more
Use the 'help' command to explore all available options
```bash
xlog help
```
Or view all options for a specific command
```bash
xlog help add
```

For more information, visit the documentation at [COMING SOON];
