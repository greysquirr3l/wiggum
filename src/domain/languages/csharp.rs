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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables, appsettings (with user-secrets locally), or Azure Key Vault — never hardcoded.",
        "All database queries must use Entity Framework parameterised queries or SqlParameter — never concatenate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (e.g. via middleware).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is added to the pipeline, not just configured.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, 3DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, bcrypt/Argon2 via BCrypt.Net-Next for passwords.",
        "Never bypass TLS certificate validation (ServerCertificateValidationCallback returning true) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `RandomNumberGenerator` — never `System.Random` for cryptographic purposes.",
        "Never use BinaryFormatter, NetDataContractSerializer, or other unsafe deserializers with untrusted data — prefer System.Text.Json or configure type allowlists.",
        "Sanitise all file paths from user input — use Path.GetFullPath and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
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

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic C# conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: avoid overly verbose XML doc comments when a brief explanation suffices.",
        "Delete tutorial-style comments that restate the code: '/// Adds two numbers' before `public int Add(int a, int b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "XML doc comments should describe contracts and edge cases, not repeat method signatures.",
    ],
};
