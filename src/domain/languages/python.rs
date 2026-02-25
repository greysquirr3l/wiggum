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
};
