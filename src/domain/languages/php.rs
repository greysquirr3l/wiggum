use super::LanguageProfile;

pub static PROFILE: &LanguageProfile = &LanguageProfile {
    build_cmd: "composer install --no-interaction --no-progress",
    test_cmd: "vendor/bin/phpunit",
    lint_cmd: "vendor/bin/php-cs-fixer fix --dry-run --diff && vendor/bin/phpstan analyse --no-progress",
    fmt_cmd: "vendor/bin/php-cs-fixer fix",

    file_extension: "php",
    manifest_file: "composer.json",

    test_file_pattern: "Test classes live alongside source in a `tests/` directory mirroring `src/` (e.g. `src/Foo.php` → `tests/FooTest.php`)",
    test_framework: "PHPUnit (or Pest) with `#[Test]`, `assertSame`, `expectException`",

    module_conventions: "One class per file. Filename matches the class (PSR-4 autoloading). Use namespaces matching directory structure; one top-level namespace per package.",
    doc_style: "PHPDoc (`/** ... */`) on classes and public methods. Include `@param`, `@return`, `@throws` types, and an example for non-trivial APIs.",
    error_handling: "Throw typed exceptions extending a package-level exception base. Catch specific exceptions at handler boundaries; never catch `\\Throwable` to swallow failures. Use `match` / `enum` for expected outcomes.",
    build_success_phrase: "All code passes PHPStan analysis without errors",

    security_rules: &[
        // Original 6 OWASP rules
        "Credentials, API keys, and secrets must only be read from environment variables (`getenv()`, `$_ENV`, or a secrets manager like Vault / AWS Secrets Manager) — never hardcoded in source files or committed `.env` files.",
        "All database queries must use PDO prepared statements with bound parameters or an ORM's safe query builder — never interpolate user input into SQL strings (SQL injection prevention).",
        "Every web application must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options response headers (via middleware or framework helper).",
        "Any controller / route that accepts user input must enforce rate limiting — verify the middleware is in the pipeline, not just configured.",
        "File upload handlers must validate MIME type server-side (e.g. `finfo_file`), reject executable extensions (`.php`, `.phtml`, `.phar`), and enforce a maximum file size.",
        "Any feature that fetches a URL on behalf of the user must validate the target against an explicit allowlist — never fetch arbitrary user-supplied URLs (SSRF prevention).",
        // Enhanced rules
        "Never use weak/broken cryptography: avoid MD5, SHA-1, DES, 3DES, RC4, or ECB mode. Use SHA-256/SHA-3 for hashing, AES-256-GCM via `sodium_crypto` for encryption, Argon2id via `password_hash` for passwords.",
        "Never disable TLS certificate verification (custom stream context with `verify_peer => false`) — fix certificate issues instead of bypassing verification.",
        "For security-sensitive random values (tokens, nonces, keys, salts), use `random_bytes` or `random_int` — never `rand`, `mt_rand`, `uniqid`, or `microtime` for cryptographic purposes.",
        "Never `unserialize()` untrusted data — it is object injection by design. Use `json_decode` with validation, or `unserialize($x, ['allowed_classes' => false])`.",
        "Never `eval()`, `assert()` on a string, `create_function()` (deprecated/removed), or `preg_replace` with the `e` modifier — prefer `eval`-free alternatives.",
        "Sanitise all file paths from user input — canonicalise via `realpath()` and verify the result stays within an allowed root (path traversal prevention).",
        "Never log sensitive data (passwords, tokens, API keys, full PAN / PII) — redact before logging or use a logger with field allowlists.",
        "Never ship TODO / FIXME comments in authentication or authorisation code — implement security checks now, not later.",
        "Generate a fresh random IV / nonce for every encryption operation — never reuse or hardcode IVs.",
        "Set cookies with the `Secure`, `HttpOnly`, and `SameSite=Lax` (or `Strict`) attributes; CSRF tokens required on all state-changing requests.",
    ],
    audit_cmd: "composer audit",

    stub_patterns: &[
        "throw new \\Exception('Not implemented')",
        "throw new \\Exception('TODO')",
        "// TODO",
        "// FIXME",
        "// HACK",
        "// XXX",
        "return null; // stub",
        "die('not implemented');",
        "echo 'not implemented';",
    ],

    wiring_hints: &[
        "Verify all service classes registered in the DI container (Symfony: `services.yaml`, Laravel: `AppServiceProvider`) are actually injected somewhere.",
        "Check that all controllers / actions have routes defined (Symfony: `routes.yaml` / attributes, Laravel: `routes/web.php` or attribute routes).",
        "Ensure all interface implementations are registered with a concrete binding in the DI container.",
        "Verify event subscribers / listeners are tagged correctly and wired (Symfony: `kernel.event_subscriber`, Laravel: `Event::listen`).",
        "Confirm middleware is added to the kernel in the correct order (`public/index.php` or framework-specific pipeline).",
        "Check that all `env()` / `config()` reads reference keys that are actually defined in `.env` / configuration files.",
    ],

    ai_avoidance_rules: &[
        "Avoid slop vocabulary: never use 'robust', 'comprehensive', 'streamlined', 'leverage', 'utilize', 'facilitate', 'innovative', 'cutting-edge', 'pivotal', 'seamless', 'synergistic', 'transformative', 'harness', 'delve', 'embark'.",
        "Avoid filler phrases: never write 'it's worth noting that', 'at its core', 'let's break this down', 'in order to', 'from a broader perspective', 'a key takeaway is', 'this underscores the importance of'.",
        "Avoid AI prompt leakage: never include phrases like 'as an AI', 'I cannot help with', 'here's the updated', 'let me explain', 'analysis:'.",
        "Write naturally: prefer short, direct sentences. Vary sentence length. Use idiomatic PHP conventions. Don't over-explain obvious things.",
        "Code should speak for itself: prefer meaningful names over comments. Only comment non-obvious decisions, safety invariants, and 'why' explanations.",
    ],

    comment_guidelines: &[
        "Only comment to explain WHY, not WHAT — the code shows what happens; comments explain non-obvious reasoning.",
        "Keep comments concise: avoid overly verbose PHPDoc when a brief description suffices.",
        "Delete tutorial-style comments that restate the code: '// This function adds two numbers' before `function add($a, $b)` is noise.",
        "Keep TODO / FIXME markers for genuine incomplete work, but never in production auth code.",
        "PHPDoc should describe contracts and edge cases, not repeat function signatures.",
    ],

    // Mirrors `docs/strict-lints.md` (PHP). Opt-in via
    // `[style] strict = true` in the plan TOML. PHPStan 2.x at `level max`
    // with strict-rules + deprecation-rules, Psalm `--taint-analysis` on
    // security-sensitive surface, PHP-CS-Fixer (PSR-12), `composer audit`
    // + Roave Security Advisories — all on PHP 8.3+.
    strict_rules: &[
        "`declare(strict_types=1);` at the top of every file — no silent scalar coercion; type every parameter, return, and property.",
        "PHPStan runs at `level max` with `phpstan-strict-rules` — at this level `mixed` is unsafe and may only be passed to another `mixed`; resolve every nullability and array-shape leak. Never weaken the level to quiet errors; use a baseline to quarantine legacy debt and block regressions only.",
        "Run Psalm `--taint-analysis` on security-sensitive surface (auth, payments, admin, any request handler) — it traces `$_GET` / `$_POST` / `$_COOKIE` sources to dangerous sinks (SQL, `echo`, `shell_exec`) and is the SAST layer PHPStan lacks.",
        "Ban dangerous functions via PHPStan `disallowedFunctionCalls` (or a CS-Fixer rule): no `eval`, `exec`, `system`, `passthru`, `shell_exec`, `assert` on strings, `extract`, `phpinfo`, and no leftover `dd` / `dump` / `var_dump`.",
        "PDO / MySQLi prepared statements with bound parameters only — never interpolate into a query string; never `PDO::query` with concatenated input.",
        "Never deserialize untrusted data with `unserialize()` (object injection) — use `json_decode` with validation, or `unserialize($x, ['allowed_classes' => false])`.",
        "Passwords via `password_hash` (Argon2id, or bcrypt) and `password_verify`; never MD5 / SHA-1 for passwords; randomness from `random_bytes` / `random_int`, never `rand` / `mt_rand` / `uniqid` for security values.",
        "Output encoding per context (`htmlspecialchars` with `ENT_QUOTES`, or the framework's auto-escaping); never `echo` raw user input. CSRF tokens on all state-changing requests; cookies `Secure`, `HttpOnly`, `SameSite`.",
        "Parse external input at the boundary into typed value objects / DTOs (e.g. `webmozart/assert`, a validator) — model domain concepts (`EmailAddress`, `Money`, `UserId`) as classes, not bare scalars; keep raw arrays out of domain internals.",
        "No `@` error suppression operator; no swallowed exceptions; catch specific types, log through the framework, never expose stack traces in production (`display_errors = off`, `log_errors = on`).",
        "Treat warnings as errors: PHP-CS-Fixer `--dry-run` and PHPStan `--no-progress` exit non-zero on any finding; Psalm `--taint-analysis` blocks the commit on tainted sinks.",
        "Pin `roave/security-advisories` as a dev dependency so Composer refuses to install a known-vulnerable package version; `composer audit` runs in preflight.",
        "No suppression without justification: never blanket-`@phpstan-ignore-next-line`; never file-scope `@psalm-suppress` waivers. Suppress narrowly, inline, with a check ID and a one-line reason. Prefer fixing.",
    ],
};
