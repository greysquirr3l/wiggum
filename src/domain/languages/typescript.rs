use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "tsc --noEmit",
    test_cmd: "vitest run",
    lint_cmd: "eslint .",
    fmt_cmd: "prettier --write .",

    file_extension: "ts",
    manifest_file: "package.json",

    test_file_pattern: "Test files are named `*.test.ts` or `*.spec.ts`, co-located with source or in a `__tests__/` directory",
    test_framework: "Vitest (or Jest) with `describe`, `it`, `expect`",

    module_conventions: "ES modules with named exports. One primary export per file. Use `index.ts` barrel files sparingly. Prefer explicit imports over re-exports.",
    doc_style: "TSDoc/JSDoc (`/** ... */`). Document public functions, interfaces, and type parameters.",
    error_handling: "Throw typed errors or return discriminated unions (`Result<T, E>` pattern). Use try/catch at boundaries. Prefer `unknown` over `any` in catch blocks.",
    build_success_phrase: "Type-checking passes without errors",

    security_rules: &[
        "Credentials, API keys, and secrets must only be read from environment variables (process.env) or a secrets manager — never hardcoded in source files or committed .env files.",
        "All database queries must use parameterised queries or an ORM's safe query builder — never interpolate user input into SQL strings (SQL injection prevention).",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (e.g. via helmet).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is actually applied to the router, not just imported.",
        "File upload handlers must validate MIME type server-side (not just file extension), reject executable types, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
    ],
    audit_cmd: "npm audit --audit-level=high",
};
