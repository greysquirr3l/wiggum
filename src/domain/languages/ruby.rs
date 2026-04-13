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

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager (e.g. credentials.yml.enc) — never hardcoded in source files.",
        "All database queries must use ActiveRecord's parameterised interface or `?` placeholders — never interpolate user input into SQL strings.",
        "Every Rack/Rails application must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options headers (e.g. via SecureHeaders gem).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted in the stack, not just configured.",
        "File upload handlers must validate MIME type server-side (e.g. via Marcel), reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "bundle exec bundler-audit check --update",

    stub_patterns: &[
        "raise NotImplementedError",
        "raise 'Not implemented'",
        "# TODO",
        "# FIXME",
        "# HACK",
        "# XXX",
        "nil # stub",
        "fail 'Not implemented'",
    ],

    wiring_hints: &[
        "Verify all classes/modules defined are actually required/autoloaded and used somewhere.",
        "Check that all controllers have routes defined in routes.rb.",
        "Ensure all services/interactors are instantiated and called from controllers or jobs.",
        "Verify ActiveJob classes are enqueued somewhere.",
        "Confirm Rack middleware is mounted in the application.",
        "Check that all initializers actually configure the services they set up.",
    ],
};
