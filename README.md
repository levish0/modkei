# modkei

Rust CLI for code statistics and file dependency graphs.

`modkei` scans a source directory, shows live per-language stats in the terminal,
and writes an interactive HTML graph of file imports.

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

- Terminal: progress spinner followed by a language table with files, lines, code, comments, and blanks.
- HTML: Svelte/Sigma graph built from generated graph data. By default `modkei` serves the output through Vite preview and opens it in the browser; use `--no-open` to only write the files.

## Architecture

The workspace is split into `modkei-core` for scanning/parsing/graph data,
`modkei-report` for static report generation, and `modkei` for CLI orchestration.
The implementation is synchronous and uses Rayon plus channels, with no async runtime.
The report pipeline writes `crates/modkei-report/ui/src/lib/generated/graph-data.json`,
runs the SvelteKit build, then copies the built HTML and assets to the requested output directory.
