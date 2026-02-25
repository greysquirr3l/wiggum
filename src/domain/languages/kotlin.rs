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
};
