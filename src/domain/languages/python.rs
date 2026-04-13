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
        "Credentials, API keys, and secrets must only be read from environment variables or a secrets manager — never hardcoded in source files or committed .env files.",
        "All database queries must use parameterised queries (e.g. `cursor.execute(sql, params)` or an ORM's safe query builder) — never use f-strings or % formatting to build SQL.",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers.",
        "Any endpoint that accepts user input must enforce rate limiting — verify the limiter is applied to the route, not just instantiated.",
        "File upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
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
};
