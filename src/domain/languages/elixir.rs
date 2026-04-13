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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from runtime config (runtime.exs / System.get_env) or a secrets manager — never hardcoded in source files.",
        "All database queries must use Ecto's parameterised query interface — never interpolate user input into raw SQL strings.",
        "Every Phoenix application must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (via plug or phoenix_secure_headers).",
        "Any controller or LiveView that accepts user input must enforce rate limiting — verify the plug is in the pipeline, not just defined.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, Argon2/bcrypt via Comeonin/Bcrypt for passwords.",
        "Never disable TLS certificate verification (:verify_none) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `:crypto.strong_rand_bytes` — never `:rand.uniform` for cryptographic purposes.",
        "Never use :erlang.binary_to_term with untrusted data without :safe option — prefer JSON or explicitly validated terms.",
        "Never use Code.eval_string or Code.eval_file with user input — prefer pattern matching and explicit validation.",
        "Sanitise all file paths from user input — use Path.expand and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use Logger metadata filtering.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "mix deps.audit",

    stub_patterns: &[
        "raise \"Not implemented\"",
        "raise \"TODO\"",
        "# TODO",
        "# FIXME",
        "# HACK",
        "# XXX",
        ":not_implemented",
        "nil # stub",
    ],

    wiring_hints: &[
        "Verify all modules are actually used/called from somewhere in the application.",
        "Check that all Phoenix controllers have routes defined in router.ex.",
        "Ensure all GenServers/Agents are started in the supervision tree.",
        "Verify all plugs defined are actually included in a pipeline.",
        "Confirm PubSub subscriptions are set up in init/1 callbacks.",
        "Check that all Ecto schemas have corresponding contexts that use them.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic Elixir conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Elixir style favors expressive code with minimal explanation.",
        "Delete tutorial-style comments that restate the code: '# This function adds two numbers' before `def add(a, b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "@doc should describe contracts and edge cases, not repeat function signatures.",
    ],
};
