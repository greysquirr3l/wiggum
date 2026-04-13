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
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables (process.env) or a secrets manager — never hardcoded in source files or committed .env files.",
        "All database queries must use parameterised queries or an ORM's safe query builder — never interpolate user input into SQL strings (SQL injection prevention).",
        "Every HTTP server must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (e.g. via helmet).",
        "Any endpoint that accepts user input must enforce rate limiting — verify the middleware is actually applied to the router, not just imported.",
        "File upload handlers must validate MIME type server-side (not just file extension), reject executable types, and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM for encryption, bcrypt/argon2 for passwords.",
        "Never disable TLS certificate verification (rejectUnauthorized: false) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys), use `crypto.randomBytes` or `crypto.getRandomValues` — never `Math.random()` for cryptographic purposes.",
        "Never use eval(), Function(), or new Function() with user input — prefer safe alternatives or validated input.",
        "Never use dangerouslySetInnerHTML without sanitising input (XSS prevention) — use DOMPurify or similar.",
        "Sanitise all file paths from user input — use path.normalize and verify paths stay within allowed directories (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys) — redact before logging or use structured logging with field allowlists.",
        "Never ship TODO/FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV/nonce for every encryption operation — never reuse or hardcode IVs.",
    ],
    audit_cmd: "npm audit --audit-level=high",

    stub_patterns: &[
        "throw new Error('Not implemented')",
        "throw new Error(\"not implemented\")",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return undefined as any",
        "return null as any",
        "return {} as any",
        "console.log('TODO:')",
    ],

    wiring_hints: &[
        "Verify all exported functions/classes from modules are actually imported and used somewhere.",
        "Check that all route handlers are registered with the Express/Fastify/Hono router.",
        "Ensure all service classes are instantiated and injected into consumers (DI container or manual).",
        "Verify event handlers are actually subscribed to their events.",
        "Confirm middleware is applied to routes (app.use() or route-level middleware).",
        "Check that all environment variables referenced in code are documented and have fallbacks.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use contractions where appropriate. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: avoid overly formal or verbose explanations.",
        "Delete tutorial-style comments that restate the code: '// This function adds two numbers' before `function add(a, b)` is noise.",
        "Keep TODO/FIXME markers for genuine incomplete work, but never in production auth code.",
        "JSDoc/TSDoc should describe contracts and edge cases, not repeat function signatures.",
    ],
};
