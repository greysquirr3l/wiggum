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
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use PreparedStatement or a type-safe query builder — never concatenate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the filter or interceptor is registered in the application context.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "mvn dependency-check:check",
};
