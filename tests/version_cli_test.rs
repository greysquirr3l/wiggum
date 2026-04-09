use assert_cmd::Command;
use predicates::str::{contains, is_match};

#[test]
fn version_subcommand_prints_version_and_git_sha() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("wiggum")?;
    let sha_pattern = is_match(r"\([0-9a-f]{7,}|unknown\)")?;

    cmd.arg("version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")))
        .stdout(sha_pattern);

    Ok(())
}
