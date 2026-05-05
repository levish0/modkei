use std::env;
use std::path::Path;
use std::process::{Command, exit};
use std::thread::sleep;
use std::time::Duration;

const CRATES: &[&str] = &["modkei-core", "modkei-report", "modkei"];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("build-ui") => build_ui().unwrap_or_else(|error| {
            eprintln!("Failed to build Svelte UI: {error}");
            exit(1);
        }),
        Some("publish") => publish(false),
        Some("publish-dry") => publish(true),
        _ => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  build-ui     Build the Svelte frontend into static HTML");
            eprintln!("  publish      Build the UI and publish crates to crates.io");
            eprintln!("  publish-dry  Dry run publish (builds UI then dry runs publish)");
            exit(1);
        }
    }
}

fn build_ui() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask directory should have a parent");
    let ui_dir = workspace_root.join("modkei-ui");

    if !ui_dir.exists() {
        return Err("UI directory not found".into());
    }

    println!("Building Svelte UI in {}...", ui_dir.display());

    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let mut build = Command::new(pnpm);
    build.current_dir(&ui_dir).arg("build");

    let status = build.status()?;
    if status.success() {
        println!("UI build completed successfully!");
        Ok(())
    } else {
        Err(format!("pnpm build failed with status {status}").into())
    }
}

fn publish(dry_run: bool) {
    println!("Preparing to publish modkei crates...\n");

    // Always build the UI first so the latest static files are embedded!
    if let Err(e) = build_ui() {
        eprintln!("Failed to build UI before publishing: {e}");
        exit(1);
    }

    for (i, crate_name) in CRATES.iter().enumerate() {
        println!("Publishing {}...", crate_name);

        let mut cmd = Command::new("cargo");
        cmd.arg("publish").arg("-p").arg(crate_name);

        if dry_run {
            cmd.arg("--dry-run");
        }

        let status = cmd.status().expect("Failed to execute cargo publish");

        if !status.success() {
            eprintln!("Failed to publish {}", crate_name);
            exit(1);
        }

        println!("{} published successfully\n", crate_name);

        // Wait for crates.io index sync so the next crate can find it (except for last crate)
        if !dry_run && i < CRATES.len() - 1 {
            println!("Waiting 15s for crates.io index sync...");
            sleep(Duration::from_secs(15));
        }
    }

    println!("All crates published!");
}
