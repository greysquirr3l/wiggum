use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "python -m py_compile $(find . -name '*.py' -not -path './.venv/*')",
    test_cmd: "pytest",
    lint_cmd: "ruff check .",
    fmt_cmd: "ruff format .",

    file_extension: "py",
    manifest_file: "pyproject.toml",

    test_file_pattern: "Test files are named `test_*.py` or `*_test.py`, typically in a `tests/` directory",
    test_framework: "pytest with `def test_*` functions and plain `assert` statements",

    module_conventions: "One module per file. Packages are directories with `__init__.py`. Use relative imports within a package. Keep `__init__.py` minimal.",
    doc_style: "Docstrings (triple-quoted) on modules, classes, and public functions. Follow Google or NumPy docstring style.",
    error_handling: "Raise specific exceptions (subclass `Exception`). Use try/except at boundaries. Avoid bare `except:`. Document raised exceptions in docstrings.",
    build_success_phrase: "All source files parse without syntax errors",

    security_rules: &[
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files or committed .env files.",
        "All database queries must use parameterised queries (e.g. `cursor.execute(sql, params)` or an ORM's safe query builder) — never use f-strings or % formatting to build SQL.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the limiter is applied to the route, not just instantiated.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, bcrypt/argon2 for passwords.",
        "Never disable TLS certificate verification (verify=False) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `secrets` module — never `random` module for cryptographic purposes.",
        "Never use eval(), exec(), or compile() with user input — prefer safe alternatives or AST-based parsing.",
        "Never use unsafe deserialisation (pickle.loads(), yaml.load() without SafeLoader) with untrusted input — use json or yaml.safe_load.",
        "Sanitise all file paths from user input — use os.path.realpath and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "pip-audit",

    stub_patterns: &[
        "raise NotImplementedError",
        "pass  # TODO",
        "# TODO",
        "# FIXME",
        "# HACK",
        "# XXX",
        "...",
        "return None  # stub",
        "raise NotImplementedError(\"Not implemented\")",
    ],

    wiring_hints: &[
        "Verify all classes/functions defined in modules are actually imported and used somewhere.",
        "Check that all route decorators (@app.route, @router.get, etc.) are registered with the application.",
        "Ensure all service classes are instantiated and injected into consumers.",
        "Verify signal handlers and event listeners are actually connected.",
        "Confirm middleware is added to the ASGI/WSGI application.",
        "Check that all environment variables referenced in code are documented in .env.example.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use Pythonic idioms. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: Python style (PEP 8) favors brief, direct comments.",
        "Delete tutorial-style comments that restate the code: '# This function adds two numbers' before `def add(a, b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "Docstrings should describe contracts and edge cases, not repeat function signatures.",
    ],
};
