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
};
