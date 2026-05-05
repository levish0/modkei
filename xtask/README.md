# modkei xtask

This directory contains the build automation and release scripts for `modkei`.
We use `xtask` to orchestrate tasks that require multiple steps, ensuring that the Svelte frontend is properly built before the Rust backend is compiled.

## Commands

### `cargo xtask build-ui`
Builds the Svelte frontend located in `crates/modkei-report/ui/`.
This runs `pnpm build` and outputs the static HTML/JS files to `crates/modkei-report/static-report/`.
**You MUST run this before compiling `modkei`**, because the Rust macro `rust-embed` bakes the contents of `static-report/` directly into the `.exe` file.

### `cargo xtask publish`
Automates the full release process to `crates.io`.
Because `modkei` depends on `modkei-report` and `modkei-core`, they must be published sequentially.
This command:
1. Runs `build-ui` to guarantee the latest UI is embedded.
2. Publishes `modkei-core`.
3. Waits 15 seconds for the crates.io index to sync.
4. Publishes `modkei-report`.
5. Waits 15 seconds.
6. Publishes `modkei`.

### `cargo xtask publish-dry`
Runs the exact same sequence as `publish`, but passes `--dry-run` to `cargo publish`. Use this to test the packaging locally before pushing to crates.io.
