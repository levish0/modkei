# modkei

Rust CLI for code statistics and file dependency graphs.

`modkei` scans a source directory, shows live per-language stats in the terminal,
and writes a standalone interactive Sigma.js HTML graph of file imports.

## Usage

```sh
cargo run -p modkei -- [PATH] [OPTIONS]
```

`PATH` defaults to the current directory.

Examples:

```sh
cargo run -p modkei --
cargo run -p modkei -- ./src --output graph.html
cargo run -p modkei -- . --no-open
```

After scanning completes, press `o` or `g` in the TUI to open the graph again.
Press `q` or `Esc` to quit.

## Options

```text
--output <file>        HTML report path (default: modkei-report.html)
--no-open             Do not open the browser automatically
--no-ignore           Do not respect ignore files
--no-ignore-parent    Do not respect ignore files in parent directories
--no-ignore-dot       Do not respect .ignore files
--no-ignore-vcs       Do not respect VCS ignore files such as .gitignore
```

## Supported Languages

Rust, TypeScript, JavaScript, Python, and Go.

## Output

- Terminal: live language table with files, lines, code, comments, and blanks.
- HTML: standalone Graphology/Sigma.js graph loaded from CDN with embedded graph JSON.

## Architecture

The workspace is split into `modkei-core` for scanning/parsing/graph data,
`modkei-tui` for the Ratatui interface, `modkei-report` for HTML generation,
and `modkei` for CLI orchestration. The implementation is synchronous and uses
Rayon plus channels, with no async runtime.
