use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "go build ./...",
    test_cmd: "go test -v ./...",
    lint_cmd: "go vet ./... && golangci-lint run ./...",
    fmt_cmd: "gofmt -w .",

    file_extension: "go",
    manifest_file: "go.mod",

    test_file_pattern: "Test files live next to source files and are named `*_test.go`",
    test_framework: "built-in (`testing` package, `func TestXxx(t *testing.T)`)",

    module_conventions: "One package per directory. Package name matches directory name. Use `internal/` for unexported packages. Keep `main` packages thin.",
    doc_style: "Godoc comments on exported symbols (`// FuncName does X.`). Package-level doc in `doc.go`.",
    error_handling: "Return `error` as the last return value. Wrap errors with `fmt.Errorf(\"context: %w\", err)`. Check errors immediately after calls.",
    build_success_phrase: "All code compiles without errors",

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries (database/sql `?` placeholders or pgx named args) — never interpolate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, and X-Frame-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted on the router, not just instantiated.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size via http.MaxBytesReader.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "govulncheck ./...",

    stub_patterns: &[
        "panic(\"not implemented\")",
        "panic(\"TODO\")",
        "panic(\"unimplemented\")",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return nil // stub",
        "return errors.New(\"not implemented\")",
    ],

    wiring_hints: &[
        "Verify all exported functions/types from packages are actually imported and used somewhere.",
        "Check that all HTTP handlers are registered with the router/mux.",
        "Ensure all interface implementations are instantiated and injected into consumers.",
        "Verify goroutines for background work are actually started (look for `go func()`).",
        "Confirm middleware is mounted on the router, not just defined.",
        "Check that all config values are actually read and used by the application.",
    ],
};
