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
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use PreparedStatement or a type-safe query builder — never concatenate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the filter or interceptor is registered in the application context.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "gradle dependencyCheckAnalyze",
};
