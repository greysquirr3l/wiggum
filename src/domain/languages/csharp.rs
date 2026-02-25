use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "dotnet build --nologo -v q",
    test_cmd: "dotnet test --nologo -v q",
    lint_cmd: "dotnet format --verify-no-changes",
    fmt_cmd: "dotnet format",

    file_extension: "cs",
    manifest_file: "*.csproj",

    test_file_pattern: "Test classes are named `*Tests.cs` in a separate test project (e.g. `*.Tests/`)",
    test_framework: "xUnit (or NUnit) with `[Fact]`, `Assert.Equal`, etc.",

    module_conventions: "One type per file, filename matches type name. Use namespaces matching folder structure. Organize by feature or layer.",
    doc_style: "XML doc comments (`/// <summary>...</summary>`) on public types and members.",
    error_handling: "Throw specific exceptions for exceptional conditions. Use result patterns or nullable returns for expected failures. Catch at service boundaries.",
    build_success_phrase: "All code compiles without errors or warnings",
};
