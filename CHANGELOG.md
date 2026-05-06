# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2026-05-06

### Changed
- Split dependency resolution out of graph construction into a dedicated `resolver` layer.
- Reworked import extraction to preserve structured raw import metadata instead of encoding import kinds in string prefixes.
- Updated package metadata and README wording to describe the project directly as source analysis plus interactive dependency visualization.

### Added
- Added language-specific parser tests under `modkei-core/src/parser/tests/`.
- Added graph resolution coverage for Rust multi-crate layouts, TypeScript path aliases, TypeScript package-local configs, Python package/file modules, Go nested modules, C/C++ includes, and `.gitignore` handling.
- Added TypeScript/JavaScript resolver support for nearest `tsconfig.json`/`jsconfig.json` `baseUrl` and `paths`.
- Added Go resolver support for nearest `go.mod` in nested module layouts.

### Fixed
- Fixed Python `from ... import ... as ...` extraction to keep imported symbols instead of aliases.
- Fixed `.gitignore` handling outside Git repositories by disabling the `ignore` crate's Git-repository requirement.
- Fixed Go module resolution so nested modules resolve relative to the nearest `go.mod` directory.

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
