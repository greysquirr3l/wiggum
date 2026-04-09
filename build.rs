//! Build script that embeds the current git SHA at compile time.
//!
//! Sets the `WIGGUM_GIT_SHA` env var, available at runtime via
//! `option_env!("WIGGUM_GIT_SHA")`.

use std::process::Command;

fn main() {
    // Re-run when the current branch or packed refs change.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.git/packed-refs");

    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(|| String::from("unknown"), |value| value.trim().to_owned());

    println!("cargo:rustc-env=WIGGUM_GIT_SHA={sha}");
}
