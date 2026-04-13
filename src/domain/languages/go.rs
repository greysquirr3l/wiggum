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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files.",
        "All database queries must use parameterised queries (database/sql `?` placeholders or pgx named args) — never interpolate user input into SQL strings.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, and X-Frame-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted on the router, not just instantiated.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size via http.MaxBytesReader.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, 3DES, RC4, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM or ChaCha20-Poly1305 for encryption, bcrypt/argon2 for passwords.",
        "Never skip TLS certificate verification (InsecureSkipVerify: true) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `crypto/rand` — never `math/rand` for cryptographic purposes.",
        "Use `encoding/json` or other safe deserializers — avoid `gob` with untrusted input as it can deserialize arbitrary types.",
        "Sanitise all file paths from user input — use `filepath.Clean` and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
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

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Go phrasing. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Go style favors brief, direct sentences without excessive formality.",
        "Delete tutorial-style comments that restate the code: '// This function adds two numbers' before `func Add(a, b int)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "Doc comments should describe contracts and edge cases, not repeat function signatures.",
    ],
};
