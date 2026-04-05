use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "mix compile --warnings-as-errors",
    test_cmd: "mix test",
    lint_cmd: "mix credo --strict",
    fmt_cmd: "mix format",

    file_extension: "ex",
    manifest_file: "mix.exs",

    test_file_pattern: "Test files are named `*_test.exs` in a `test/` directory, mirroring `lib/` structure",
    test_framework: "ExUnit with `test \"description\" do`, `assert`, `assert_receive`, etc.",

    module_conventions: "One module per file. Filename matches module name in snake_case. Use nested modules for namespacing. Keep `application.ex` and supervision trees at the top level.",
    doc_style: "Module doc (`@moduledoc`) and function doc (`@doc`) with Markdown. Add `@spec` typespecs for public functions.",
    error_handling: "Return `{:ok, value}` / `{:error, reason}` tuples for expected outcomes. Use `with` for chaining. Reserve `raise` for truly exceptional conditions.",
    build_success_phrase: "All code compiles without errors or warnings",

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from runtime config (runtime.exs / System.get_env) or a secrets manager — never hardcoded in source files.",
        "All database queries must use Ecto's parameterised query interface — never interpolate user input into raw SQL strings.",
        "Every Phoenix application must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (via plug or phoenix_secure_headers).",
        "Any controller or LiveView that accepts user input must enforce rate limiting — verify the plug is in the pipeline, not just defined.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "mix deps.audit",
};
