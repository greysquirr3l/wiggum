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
};
