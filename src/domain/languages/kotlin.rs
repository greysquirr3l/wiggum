use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "gradle build -x test -q",
    test_cmd: "gradle test",
    lint_cmd: "gradle detekt",
    fmt_cmd: "gradle ktlintFormat",

    file_extension: "kt",
    manifest_file: "build.gradle.kts",

    test_file_pattern: "Test classes are named `*Test.kt` in `src/test/kotlin/`, mirroring the main source package structure",
    test_framework: "JUnit 5 (or kotest) with `@Test`, `assertEquals`, etc.",

    module_conventions: "Multiple classes per file are okay when related. Use packages matching directory structure. Prefer top-level functions over utility classes.",
    doc_style: "KDoc (`/** ... */`) on public APIs. Use `@param`, `@return`, `@throws` tags. Supports Markdown in doc comments.",
    error_handling: "Use sealed classes or `Result<T>` for expected failures. Throw exceptions for unexpected errors. Prefer `runCatching` over try/catch for functional style.",
    build_success_phrase: "All code compiles without errors",

    security_rules: &[
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use PreparedStatement or a type-safe query builder — never concatenate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the filter or interceptor is registered in the application context.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, 3DES, RC4, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, bcrypt/Argon2 for passwords.",
        "Never disable TLS certificate verification (ALLOW_ALL_HOSTNAME_VERIFIER, TrustAllCerts) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `SecureRandom` — never `kotlin.random.Random` for cryptographic purposes.",
        "Never use ObjectInputStream.readObject() with untrusted data — prefer kotlinx.serialization with JSON or configure deserialisation allowlists.",
        "Sanitise all file paths from user input — use Path.normalize() and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "gradle dependencyCheckAnalyze",

    stub_patterns: &[
        "TODO()",
        "throw NotImplementedError()",
        "throw UnsupportedOperationException()",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return null // stub",
        "/* TODO */",
    ],

    wiring_hints: &[
        "Verify all @Component, @Service, @Repository beans are actually injected somewhere.",
        "Check that all @Controller/@RestController endpoints are reachable and have proper request mappings.",
        "Ensure all interface implementations have corresponding @Bean definitions or @Component annotations.",
        "Verify @Scheduled methods are in beans managed by the application context.",
        "Confirm Ktor routing modules are actually installed in the application.",
        "Check that all configuration properties are bound to actual config files.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Kotlin conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Kotlin style favors brief, expressive code with minimal explanation.",
        "Delete tutorial-style comments that restate the code: '// This function adds two numbers' before `fun add(a: Int, b: Int)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "KDoc should describe contracts and edge cases, not repeat function signatures.",
    ],
};
