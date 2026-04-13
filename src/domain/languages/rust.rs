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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries (e.g. sqlx's `query!` macro) — never interpolate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, and X-Frame-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is wired to the router, not just defined.",
        "File upload handlers must validate MIME type server-side, reject executable extensions (.exe, .sh, .php, etc.), and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules from papertowel analysis
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, 3DES, RC4, Blowfish, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM or ChaCha20-Poly1305 for encryption, Argon2id/bcrypt/scrypt for passwords.",
        "Never disable TLS certificate verification (e.g. danger_accept_invalid_certs) — fix root certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `rand::rngs::OsRng` or `getrandom` — never `rand::thread_rng()` for cryptographic purposes.",
        "Never use unsafe deserialisation on untrusted data — prefer `serde_json` over formats that allow arbitrary code execution.",
        "Sanitise all file paths from user input — canonicalise paths and verify they stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
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

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use contractions where appropriate. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Preserve safety/security comments: keep comments containing 'Safety:', 'SAFETY:', 'security', 'invariant', 'caveat', 'trade-off'.",
        "Delete tutorial-style comments that restate the code: '// This function adds two numbers' before `fn add(a, b)` is noise.",
        "Keep TODO/FIXME/HACK markers for genuine incomplete work, but never in production auth code.",
        "Doc comments (`///`) should describe contracts and edge cases, not repeat function signatures.",
    ],
};
