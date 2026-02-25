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
};
