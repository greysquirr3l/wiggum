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
};
