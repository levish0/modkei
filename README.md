# modkei

**modkei** is a blazing fast Rust CLI tool for exploring code statistics and file dependency graphs. 

It scans your source directory, prints a `tokei`-like breakdown of lines of code by language, and automatically opens a beautiful, interactive node-graph visualization of your file dependencies in the browser. 

> ✨ **Inspired by Obsidian**: The visualizer is heavily inspired by Obsidian's interactive graph view, helping you intuitively understand your codebase's architecture and module relationships at a glance.

## Installation

Install `modkei` globally via crates.io:

```sh
cargo install modkei
```

## Usage

Navigate to any project directory and run:

```sh
modkei .
```

`modkei` will:
1. Scan the directory (respecting your `.gitignore`).
2. Print the file statistics to the terminal.
3. Spin up an embedded local server and automatically open the interactive dependency graph in your web browser.

### Options

```text
Usage: modkei [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to analyze [default: .]

Options:
  --no-open             Do not open the browser automatically
  --no-ignore           Do not respect ignore files
  --no-ignore-parent    Do not respect ignore files in parent directories
  --no-ignore-dot       Do not respect .ignore files
  --no-ignore-vcs       Do not respect VCS ignore files such as .gitignore
  -h, --help            Print help
  -V, --version         Print version
```

## Supported Languages

`modkei` uses `tree-sitter` for robust parsing and include/import resolution. Currently supported languages:

- Rust
- TypeScript / JavaScript
- Python
- Go
- C / C++
- Java
- Bash
- Makefile
- CMake

## Architecture & Development

The workspace is split into three core crates:
- `modkei-core`: Multi-threaded scanning, parsing, and graph data generation using `rayon` and `tree-sitter`.
- `modkei-report`: In-memory `tiny-http` server that serves the static Svelte frontend baked into the binary via `rust-embed`.
- `modkei`: The CLI orchestration tool.

The interactive graph UI is built with **SvelteKit**, **Sigma.js**, and **D3-force**, located in the `modkei-ui` directory.

### Building from Source

To build `modkei` locally, you must first build the Svelte frontend into static HTML files so they can be embedded into the Rust binary. We use a custom `xtask` workflow for this:

```sh
# 1. Build the UI (requires Node.js & pnpm)
cargo xtask build-ui

# 2. Build the Rust CLI
cargo build --release
```
