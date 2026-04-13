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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager (e.g. credentials.yml.enc) — never hardcoded in source files.",
        "All database queries must use ActiveRecord's parameterised interface or `?` placeholders — never interpolate user input into SQL strings.",
        "Every Rack/Rails application must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options headers (e.g. via SecureHeaders gem).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is mounted in the stack, not just configured.",
        "File upload handlers must validate MIME type server-side (e.g. via Marcel), reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, bcrypt for passwords.",
        "Never disable TLS certificate verification (verify_mode = OpenSSL::SSL::VERIFY_NONE) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `SecureRandom` — never `rand` for cryptographic purposes.",
        "Never use Marshal.load or YAML.load with untrusted data — prefer JSON or YAML.safe_load.",
        "Never use eval, instance_eval, or class_eval with user input — prefer safe metaprogramming patterns.",
        "Sanitise all file paths from user input — use File.expand_path and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — use Rails filter_parameters or redact before logging.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
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

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Ruby conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Ruby style favors expressive code with minimal explanation.",
        "Delete tutorial-style comments that restate the code: '# This method adds two numbers' before `def add(a, b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "YARD comments should describe contracts and edge cases, not repeat method signatures.",
    ],
};
