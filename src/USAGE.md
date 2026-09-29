# Using OxidLog

## Basic usage

* `xlog init` - Create a new journal, will overwrite an existing one.
* `xlog add "my first #note"` - Will create a new entry, with `note` as a tag.
* `xlog view` - List all entries
* `xlog remove 0` - Remove entry, with id 0.

## Advanced usage and arguments
Run `xlog help` to get an overview of all the available commands.
You can also run `xlog help [command]` to view all of the options in detail that can be passed.

## Config
There is a config file automatically created at `$XDG_CONFIG_HOME/oxidlog/config.toml` (default `~/.config/oxidlog/`); the journal lives in `$XDG_DATA_HOME/oxidlog/` (default `~/.local/share/oxidlog/`). Set `XLOG_HOME` to keep both in one directory.

[More docs are coming soon] (cap)