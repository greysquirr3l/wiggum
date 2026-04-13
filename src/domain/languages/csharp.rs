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

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from environment variables, appsettings (with user-secrets locally), or Azure Key Vault — never hardcoded.",
        "All database queries must use Entity Framework parameterised queries or SqlParameter — never concatenate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (e.g. via middleware).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is added to the pipeline, not just configured.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "dotnet list package --vulnerable",

    stub_patterns: &[
        "throw new NotImplementedException()",
        "throw new NotSupportedException()",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return default; // stub",
        "/* TODO */",
    ],

    wiring_hints: &[
        "Verify all services registered in DI container (AddScoped, AddSingleton, AddTransient) are actually injected somewhere.",
        "Check that all controller endpoints are reachable and have proper route attributes.",
        "Ensure all interface implementations are registered in the DI container.",
        "Verify hosted services (IHostedService) are registered with AddHostedService.",
        "Confirm middleware is added to the pipeline in the correct order in Program.cs.",
        "Check that all configuration sections (IOptions<T>) are bound to appsettings.json sections.",
    ],
};
