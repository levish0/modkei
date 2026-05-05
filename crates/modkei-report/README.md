# modkei-report

`ui/` is reserved for the SvelteKit report app.

Expected flow:

```sh
cd crates/modkei-report/ui
# initialize SvelteKit here
# build/export the standalone static report assets into ../static-report
```

`static-report/` is the Rust-facing build output directory. The report crate can
later embed those files and inject the generated graph JSON into the HTML entry.
