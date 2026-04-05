mod csharp;
mod elixir;
mod go;
mod java;
mod kotlin;
mod python;
mod ruby;
mod rust;
mod swift;
mod typescript;

use super::plan::Language;

/// Language-specific profile containing tool commands, conventions,
/// and best practices that get injected into generated templates.
#[derive(Debug, Clone)]
pub struct LanguageProfile {
    // ─── Toolchain commands ──────────────────────────────────────
    /// Command to build / type-check the project.
    pub build_cmd: &'static str,
    /// Command to run the test suite.
    pub test_cmd: &'static str,
    /// Command to lint the project.
    pub lint_cmd: &'static str,
    /// Command to format the project (informational).
    pub fmt_cmd: &'static str,

    // ─── File conventions ────────────────────────────────────────
    /// Primary source file extension (e.g. "rs", "py").
    pub file_extension: &'static str,
    /// Manifest file name used for project detection (e.g. "Cargo.toml").
    pub manifest_file: &'static str,

    // ─── Testing conventions ─────────────────────────────────────
    /// How test files are typically named or located.
    pub test_file_pattern: &'static str,
    /// Default test framework name.
    pub test_framework: &'static str,

    // ─── Module / package conventions ────────────────────────────
    /// Brief description of module/package structure conventions.
    pub module_conventions: &'static str,

    // ─── Documentation style ─────────────────────────────────────
    /// How documentation is written in this language.
    pub doc_style: &'static str,

    // ─── Error handling ──────────────────────────────────────────
    /// Idiomatic error handling approach.
    pub error_handling: &'static str,

    // ─── Exit criteria wording ───────────────────────────────────
    /// How to phrase "code compiles" for this language.
    pub build_success_phrase: &'static str,
    // ─── Security ────────────────────────────────────────────────
    /// Non-negotiable OWASP-derived security rules injected into every
    /// generated subagent prompt alongside the user-supplied rules.
    /// These rules are language-specific but cover the same universal
    /// risk categories: secrets, injection, rate limiting, uploads,
    /// security headers, and SSRF.
    pub security_rules: &'static [&'static str],

    /// Command to run a supply-chain / vulnerability audit (e.g. `cargo audit`).
    /// Appended to every preflight check after lint. Empty string = skipped.
    pub audit_cmd: &'static str,
}

/// Get the profile for a language.
#[must_use]
pub fn profile(language: Language) -> &'static LanguageProfile {
    match language {
        Language::Rust => rust::PROFILE,
        Language::Go => go::PROFILE,
        Language::TypeScript => typescript::PROFILE,
        Language::Python => python::PROFILE,
        Language::Java => java::PROFILE,
        Language::CSharp => csharp::PROFILE,
        Language::Kotlin => kotlin::PROFILE,
        Language::Swift => swift::PROFILE,
        Language::Ruby => ruby::PROFILE,
        Language::Elixir => elixir::PROFILE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_languages_have_profiles() {
        for lang in Language::ALL {
            let p = profile(*lang);
            assert!(!p.build_cmd.is_empty(), "{lang} missing build_cmd");
            assert!(!p.test_cmd.is_empty(), "{lang} missing test_cmd");
            assert!(!p.lint_cmd.is_empty(), "{lang} missing lint_cmd");
            assert!(!p.manifest_file.is_empty(), "{lang} missing manifest_file");
            assert!(
                !p.file_extension.is_empty(),
                "{lang} missing file_extension"
            );
        }
    }

    #[test]
    fn rust_profile_values() {
        let p = profile(Language::Rust);
        assert!(p.build_cmd.contains("cargo"));
        assert!(p.test_cmd.contains("cargo"));
        assert!(p.lint_cmd.contains("clippy"));
        assert_eq!(p.file_extension, "rs");
        assert_eq!(p.manifest_file, "Cargo.toml");
    }

    #[test]
    fn python_profile_values() {
        let p = profile(Language::Python);
        assert_eq!(p.file_extension, "py");
        assert_eq!(p.manifest_file, "pyproject.toml");
        assert!(p.test_cmd.contains("pytest"));
    }

    #[test]
    fn go_profile_values() {
        let p = profile(Language::Go);
        assert_eq!(p.file_extension, "go");
        assert_eq!(p.manifest_file, "go.mod");
    }

    #[test]
    fn all_profiles_have_non_empty_conventions() {
        for lang in Language::ALL {
            let p = profile(*lang);
            assert!(
                !p.module_conventions.is_empty(),
                "{lang} missing module_conventions"
            );
            assert!(!p.doc_style.is_empty(), "{lang} missing doc_style");
            assert!(
                !p.error_handling.is_empty(),
                "{lang} missing error_handling"
            );
            assert!(
                !p.build_success_phrase.is_empty(),
                "{lang} missing build_success_phrase"
            );
        }
    }
}
