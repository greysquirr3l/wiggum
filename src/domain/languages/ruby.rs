use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "ruby -c $(find . -name '*.rb' -not -path './vendor/*')",
    test_cmd: "bundle exec rspec",
    lint_cmd: "bundle exec rubocop",
    fmt_cmd: "bundle exec rubocop -A",

    file_extension: "rb",
    manifest_file: "Gemfile",

    test_file_pattern: "Test files are named `*_spec.rb` in a `spec/` directory (RSpec) or `*_test.rb` in `test/` (Minitest)",
    test_framework: "RSpec with `describe`, `it`, `expect` (or Minitest with `def test_*`)",

    module_conventions: "One class per file. Filename matches class name in snake_case. Use modules for namespacing. Require files explicitly or use autoloading (Zeitwerk).",
    doc_style: "YARD doc comments (`# @param`, `# @return`) or plain comments above methods. Use `README.md` for high-level documentation.",
    error_handling: "Raise specific exceptions (subclass `StandardError`). Rescue at boundaries. Avoid bare `rescue`. Use custom error hierarchies per domain.",
    build_success_phrase: "All source files parse without syntax errors",
};
