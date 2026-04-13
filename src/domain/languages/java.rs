use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "mvn compile -q",
    test_cmd: "mvn test -q",
    lint_cmd: "mvn checkstyle:check -q",
    fmt_cmd: "mvn spotless:apply",

    file_extension: "java",
    manifest_file: "pom.xml",

    test_file_pattern: "Test classes are named `*Test.java` in `src/test/java/`, mirroring the main source package structure",
    test_framework: "JUnit 5 with `@Test`, `Assertions.assertEquals`, etc.",

    module_conventions: "One public class per file, filename matches class name. Group by feature package. Use `module-info.java` for Java modules.",
    doc_style: "Javadoc (`/** ... */`) on public classes, methods, and fields. Include `@param`, `@return`, `@throws` tags.",
    error_handling: "Use checked exceptions for recoverable errors, unchecked for programming bugs. Catch at appropriate boundaries. Never swallow exceptions silently.",
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
        "For security-sensitive random values (tokens, nonces, keys), use `SecureRandom` — never `java.util.Random` for cryptographic purposes.",
        "Never use ObjectInputStream.readObject() with untrusted data — prefer JSON/XML or configure ObjectInputFilter allowlists.",
        "Sanitise all file paths from user input — use Path.normalize() and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "mvn dependency-check:check",

    stub_patterns: &[
        "throw new UnsupportedOperationException()",
        "throw new UnsupportedOperationException(\"Not implemented\")",
        "throw new RuntimeException(\"TODO\")",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return null; // stub",
        "/* TODO */",
    ],

    wiring_hints: &[
        "Verify all @Component, @Service, @Repository beans are actually injected somewhere via @Autowired or constructor injection.",
        "Check that all @Controller/@RestController endpoints are reachable and have proper request mappings.",
        "Ensure all interface implementations have corresponding @Bean definitions or @Component annotations.",
        "Verify @Scheduled methods are in beans managed by Spring context.",
        "Confirm @EventListener methods are in beans that are component-scanned.",
        "Check that all configuration properties classes are actually bound to properties files.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Java conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: avoid overly formal or verbose Javadoc when a brief explanation suffices.",
        "Delete tutorial-style comments that restate the code: '// This method adds two numbers' before `public int add(int a, int b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "Javadoc should describe contracts and edge cases, not repeat method signatures.",
    ],
};
