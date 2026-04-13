use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "swift build",
    test_cmd: "swift test",
    lint_cmd: "swiftlint lint --strict",
    fmt_cmd: "swift-format format -i -r Sources/ Tests/",

    file_extension: "swift",
    manifest_file: "Package.swift",

    test_file_pattern: "Test classes are named `*Tests.swift` in a `Tests/` directory, organized by target",
    test_framework: "XCTest (or Swift Testing) with `func testXxx()`, `XCTAssertEqual`, etc.",

    module_conventions: "One type per file is conventional. Use Swift Package Manager targets for modularity. Access control via `public`, `internal` (default), `private`.",
    doc_style: "Swift doc comments (`///` or `/** */`) with Markdown. Use `- Parameters:`, `- Returns:`, `- Throws:` callouts.",
    error_handling: "Use `throws` functions with typed errors. Handle with `do/try/catch`. Use `Result<T, E>` for async or callback-based APIs.",
    build_success_phrase: "All code compiles without errors or warnings",

    security_rules: &[
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from the Keychain, environment variables, or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries or a type-safe query builder — never format user input into SQL strings.",
        "Any HTTP server component must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted on the router.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM or ChaChaPoly for encryption, bcrypt for passwords.",
        "Never disable TLS certificate verification (allow invalid certificates in URLSession) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `SecRandomCopyBytes` or CryptoKit — never SystemRandomNumberGenerator for cryptographic purposes.",
        "Never use NSKeyedUnarchiver without secure coding on untrusted data — use Codable with JSON/Property Lists instead.",
        "Sanitise all file paths from user input — use URL path resolution and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use os_log with appropriate privacy levels.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "",

    stub_patterns: &[
        "fatalError(\"Not implemented\")",
        "fatalError(\"TODO\")",
        "preconditionFailure(\"Not implemented\")",
        "// TODO:",
        "// FIXME:",
        "// HACK:",
        "// XXX:",
        "return nil // stub",
        "/* TODO */",
    ],

    wiring_hints: &[
        "Verify all public types/functions from modules are actually imported and used somewhere.",
        "Check that all route handlers are registered with Vapor/Hummingbird router.",
        "Ensure all protocol implementations are instantiated and passed to consumers.",
        "Verify async tasks are actually awaited and not fire-and-forget.",
        "Confirm middleware is added to the application's middleware stack.",
        "Check that all environment variables are documented and have fallback values.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Swift conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Swift style favors expressive code with minimal explanation.",
        "Delete tutorial-style comments that restate the code: '/// This function adds two numbers' before `func add(_ a: Int, _ b: Int)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "Doc comments should describe contracts and edge cases, not repeat function signatures.",
    ],
};
