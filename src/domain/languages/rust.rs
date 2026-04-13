use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "cargo build --workspace",
    test_cmd: "cargo test --workspace",
    lint_cmd: "cargo clippy --workspace -- -D warnings",
    fmt_cmd: "cargo fmt --all",

    file_extension: "rs",
    manifest_file: "Cargo.toml",

    test_file_pattern: "Tests live alongside source in `#[cfg(test)] mod tests` blocks, or in a top-level `tests/` directory for integration tests",
    test_framework: "built-in (`#[test]`, `assert!`, `assert_eq!`)",

    module_conventions: "One module per file. `mod.rs` or `<name>.rs` declares sub-modules. Use `pub(crate)` for internal visibility. Re-export public API from `lib.rs`.",
    doc_style: "Rustdoc (`///` for items, `//!` for module-level). Include `# Examples`, `# Errors`, `# Panics` sections where relevant.",
    error_handling: "Use `Result<T, E>` with a crate-level error enum (via `thiserror`). Propagate with `?`. Avoid `.unwrap()` and `.expect()` outside tests.",
    build_success_phrase: "All code compiles without errors or warnings",

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries (e.g. sqlx's `query!` macro) — never interpolate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, and X-Frame-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is wired to the router, not just defined.",
        "File upload handlers must validate MIME type server-side, reject executable extensions (.exe, .sh, .php, etc.), and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "cargo audit",

    stub_patterns: &[
        "todo!()",
        "unimplemented!()",
        "panic!(\"not implemented\")",
        "panic!(\"unimplemented\")",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "Default::default() // placeholder",
        "unreachable!(\"stub\")",
    ],

    wiring_hints: &[
        "Verify all `pub` items in `lib.rs` are actually imported/used somewhere in the binary or tests.",
        "For hexagonal architecture: confirm every port trait has at least one adapter implementation wired in `main.rs`.",
        "Check that all route handlers are registered with the router — grep for handler function names in route definitions.",
        "Ensure all `impl` blocks for traits are instantiated and passed to consumers.",
        "Verify background workers/tasks are spawned (look for `tokio::spawn` or `thread::spawn` calls).",
        "Confirm error types are properly converted — look for `From` impls or `.map_err()` chains.",
        "Check that all configured features/endpoints in config are actually wired to implementations.",
    ],
};
