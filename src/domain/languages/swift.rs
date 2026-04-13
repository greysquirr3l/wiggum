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
        "Credentials, API keys, and secrets must only be read from the Keychain, environment variables, or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries or a type-safe query builder — never format user input into SQL strings.",
        "Any HTTP server component must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted on the router.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
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
};
