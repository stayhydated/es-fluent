# es-fluent-cli

`es-fluent-cli` is a command-line tool that provides a Terminal User Interface (TUI) for managing Fluent translations. It automatically discovers crates with `i18n.toml` configuration, generates `.ftl` files, and watches for changes in your source code to trigger rebuilds.

This tool is ideal for a "watch" mode during development, giving you instant feedback and generating translation keys as you write your code.

## Usage

To start the CLI in its default watch mode, simply run the following command in your project's root directory:

```sh
es-fluent-cli
```

The tool will scan for crates, perform an initial build, and then monitor for file changes.

### Modes

You can control how the `.ftl` files are generated using the `--mode` flag.

- **Conservative Mode (Default)**: Adds new translation keys to your `.ftl` files while preserving any existing keys and their translations. This is the safest and default mode.

  ```sh
  es-fluent-cli --mode conservative
  ```

- **Aggressive Mode**: Overwrites the existing `.ftl` file entirely with newly generated keys. This is useful for cleaning up stale keys but will erase any manual changes or translations in the file.

  ```sh
  es-fluent-cli --mode aggressive
  ```
