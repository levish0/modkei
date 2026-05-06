# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-05-06

### Changed
- Narrowed the analyzer scope to Rust so dependency graph quality is driven by Rust semantics instead of multi-language tree-sitter heuristics.
- Replaced the Rust dependency graph path with a `rust-analyzer` backend for workspace-aware resolution.
- Kept a Rust syntax fallback for manifest-less module trees such as ad-hoc `src/lib.rs` + `mod foo;` layouts.
- Updated package metadata and README wording to describe the project as a Rust codebase analyzer.

### Added
- Added Rust graph coverage for manifest-less module files, `mod.rs` directories, workspace member resolution, and default `.gitignore` handling.

## [0.1.4] - 2025-05-06
Bump workspace version to 0.1.4

## [0.1.3] - 2025-05-06
Bump workspace version to 0.1.3

## [0.1.2] - 2025-05-06
Bump workspace version to 0.1.2

## [0.1.1] - 2025-05-06
Bump workspace and member crate versions to 0.1.1 and update package metadata. Adds keywords and categories to the workspace Cargo.toml and enables description/readme/keywords/categories workspace fields for modkei-core, modkei-report, and modkei to populate crate metadata (including brief descriptions for core and report). Cargo.lock updated to reflect the new versions.

## [0.1.0] - 2026-05-06

### Added
- **Core CLI (`modkei`)**: Initial release of the `modkei` executable that explores code statistics and file dependencies.
- **Dependency Graph Analysis (`modkei-core`)**: Fast, multi-threaded codebase scanning honoring `.gitignore` rules, powered by tree-sitter.
- **Multi-Language Support**: Parsers and robust import/include resolution for:
  - Rust, TypeScript, JavaScript, Python, Go, C, C++, Java, Bash, Makefile, and CMake.
- **Interactive Visual Report (`modkei-ui`)**: A responsive node-graph visualization built with SvelteKit, Sigma.js, and D3-force. Features include:
  - Dynamic physics controls (Repel Force, Link Distance, Center Force).
  - Node filtering (Hide orphans, text fade threshold).
  - Search functionality and focused node highlighting.
- **Single-Binary Architecture**: The Svelte UI is statically built and baked directly into the Rust executable using `rust-embed`, served locally via `tiny-http`. No Node.js dependencies are required for users.
- **Automated Workflows (`xtask`)**: Custom `cargo xtask` commands:
  - `build-ui`: Compiles the Svelte frontend into static HTML for Rust embedding.
  - `publish`: Automates sequential crate publishing to crates.io (`modkei-core` -> `modkei-report` -> `modkei`).
- **CI/CD Integration**: GitHub Actions workflows for linting (Clippy), formatting (Rustfmt), and synchronized UI & Rust builds.

### Changed
- Transitioned from an external `npx serve` frontend dependency to a standalone memory-based `tiny-http` server.
- Extracted frontend source from `crates/modkei-report/ui` to the workspace root (`modkei-ui`) for cleaner project encapsulation.
- Optimized D3 physics simulations to exclude hidden/orphan nodes from calculations, drastically improving graph rendering performance on large codebases.
