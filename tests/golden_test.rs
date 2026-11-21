//! Golden plan corpus: every `tests/golden/*.toml` plan is generated into a
//! temp dir (all targets) and checked for plan quality, the Stop and ask
//! section, a command in every task's Exit Criteria, and unrendered template
//! markers.
//!
//! Adding a plan is enough: drop a `*.toml` into `tests/golden/`. Each plan
//! carries its own score floor in a leading `# golden-floor: N` comment. To
//! bump a floor after improving a plan, set N to the new `check` score (the
//! failure message and `wiggum check <plan>` both print it). Do not lower a
//! floor to make a failure go away; fix the plan or the generator.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use wiggum::adapters::fs::FsAdapter;
use wiggum::domain::check::score_plan;
use wiggum::domain::dag::validate_dag;
use wiggum::domain::plan::{Plan, ResolvedTask, validate_gates};
use wiggum::domain::targets::TargetSet;
use wiggum::generation;

/// Built-in default entries of the Stop and ask section, in order. Kept
/// literal here on purpose: the test must fail if the generator's defaults
/// change without the golden corpus being reviewed.
const DEFAULT_STOP_LINES: [&str; 5] = [
    "- before deleting important data",
    "- before force-pushing",
    "- before modifying external systems",
    "- before any other destructive operation",
    "- before a decision that needs the human's approval",
];

/// Generated files that must carry the Stop and ask section.
const STOP_SECTION_FILES: [&str; 4] = [
    ".vscode/orchestrator.prompt.md",
    ".opencode/agents/orchestrator.md",
    "ORCHESTRATOR.md",
    "CLAUDE.md",
];

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn golden_plans() -> Vec<PathBuf> {
    let mut plans: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .expect("tests/golden must exist")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    plans.sort();
    plans
}

/// Parse the `# golden-floor: N` comment. It must be on a leading comment line.
fn parse_floor(toml: &str) -> Option<u8> {
    toml.lines()
        .take_while(|l| l.trim_start().starts_with('#'))
        .find_map(|l| l.trim_start().strip_prefix("# golden-floor:"))
        .and_then(|v| v.trim().parse().ok())
}

fn all_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The body lines of a `## <heading>` section, up to the next `## ` heading.
fn section_lines<'a>(text: &'a str, heading: &str) -> Option<Vec<&'a str>> {
    let mut lines = text.lines();
    lines.by_ref().find(|l| *l == heading)?;
    Some(lines.take_while(|l| !l.starts_with("## ")).collect())
}

/// Stop and ask: the whole section must appear as an exact contiguous block.
fn check_stop_section(plan: &Plan, rel: &str, text: &str, errors: &mut Vec<String>) {
    let mut expected: Vec<String> = vec![
        "## Stop and ask".into(),
        String::new(),
        "Stop and ask the human:".into(),
        String::new(),
    ];
    expected.extend(DEFAULT_STOP_LINES.iter().map(|s| (*s).to_string()));
    expected.extend(plan.orchestrator.stop_when.iter().map(|s| format!("- {s}")));
    expected.push(String::new());
    expected.push("Keep working when the next step does not need the human's input.".into());

    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| *l == "## Stop and ask") else {
        errors.push(format!("{rel}: missing `## Stop and ask` heading line"));
        return;
    };
    let actual: Vec<&str> = lines
        .iter()
        .skip(start)
        .take(expected.len())
        .copied()
        .collect();
    let want: Vec<&str> = expected.iter().map(String::as_str).collect();
    if actual != want {
        errors.push(format!(
            "{rel}: Stop and ask section differs from expected.\n--- expected\n{}\n--- actual\n{}",
            want.join("\n"),
            actual.join("\n")
        ));
    }
}

/// What counts as "a command in Exit Criteria" for one task file:
///
/// * the task declares `verify` commands: every one must appear in the
///   `## Exit Criteria` section as the whole line
///   "- [ ] Verify: `<cmd>` exits with status 0" (backticks around the
///   command); or
/// * the task declares none: the `## Preflight` section must hold a fenced
///   ```` ```bash ```` block with exactly one non-empty command line (the
///   preflight command the Exit Criteria are verified with).
fn check_task_commands(task: &ResolvedTask, rel: &str, text: &str, errors: &mut Vec<String>) {
    if task.verify.is_empty() {
        let Some(pre) = section_lines(text, "## Preflight") else {
            errors.push(format!("{rel}: no `## Preflight` section"));
            return;
        };
        let cmd_ok = matches!(pre.as_slice(),
            ["", "```bash", cmd, "```", ..] if !cmd.trim().is_empty());
        if !cmd_ok {
            errors.push(format!(
                "{rel}: Preflight section has no fenced bash command; got {pre:?}"
            ));
        }
        if section_lines(text, "## Exit Criteria").is_none() {
            errors.push(format!("{rel}: no `## Exit Criteria` section"));
        }
        return;
    }
    let Some(exit) = section_lines(text, "## Exit Criteria") else {
        errors.push(format!("{rel}: no `## Exit Criteria` section"));
        return;
    };
    for cmd in &task.verify {
        let want = format!("- [ ] Verify: `{cmd}` exits with status 0");
        if !exit.contains(&want.as_str()) {
            errors.push(format!("{rel}: Exit Criteria lacks whole line `{want}`"));
        }
    }
}

/// Unrendered Tera markers. No generated file legitimately contains `{{` or
/// `{%` today (checked when this test was written), so there is no allowlist:
/// any hit is a failure. If a future template must document Tera syntax,
/// add an explicit, justified per-file exception here rather than loosening
/// the scan.
fn check_markers(rel: &str, text: &str, errors: &mut Vec<String>) {
    for (i, line) in text.lines().enumerate() {
        for marker in ["{{", "{%"] {
            if line.contains(marker) {
                errors.push(format!(
                    "{rel}:{}: unrendered template marker `{marker}`: {line}",
                    i + 1
                ));
            }
        }
    }
}

fn check_plan(path: &Path) -> Vec<String> {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mut errors = Vec::new();
    let mut err = |msg: String| errors.push(format!("[{name}] {msg}"));

    let toml = std::fs::read_to_string(path).expect("read plan");
    let Some(floor) = parse_floor(&toml) else {
        err("missing leading `# golden-floor: N` comment".into());
        return errors;
    };
    let plan = match Plan::from_toml(&toml) {
        Ok(p) => p,
        Err(e) => {
            err(format!("plan does not parse: {e}"));
            return errors;
        }
    };

    // Same validation `wiggum generate` runs.
    let resolved = match plan.resolve_tasks() {
        Ok(r) => r,
        Err(e) => {
            err(format!("resolve_tasks failed: {e}"));
            return errors;
        }
    };
    let validation = validate_dag(&resolved)
        .map(|_| ())
        .and_then(|()| plan.validate_gates_and_evaluator(&resolved))
        .and_then(|()| validate_gates(&plan, &resolved))
        .and_then(|()| plan.validate_capabilities(&resolved));
    if let Err(e) = validation {
        err(format!("plan is not valid for `wiggum generate`: {e}"));
        return errors;
    }

    // 1. Score floor.
    let score = score_plan(&plan, &resolved).overall;
    if score < floor {
        err(format!(
            "check score {score} is below recorded floor {floor}"
        ));
    }

    // Generate everything (all targets) into a temp dir; nothing is written to
    // the plan's own `[project].path`.
    let out = TempDir::new().expect("temp dir");
    let generated = generation::generate_all_with_overrides(&plan, out.path())
        .and_then(|a| generation::write_artifacts(&FsAdapter, out.path(), &a, &TargetSet::all()));
    if let Err(e) = generated {
        err(format!("generation failed: {e}"));
        return errors;
    }

    let read = |rel: &str| std::fs::read_to_string(out.path().join(rel));
    let mut file_errors = Vec::new();

    // 2. Stop and ask in every orchestrator variant.
    for rel in STOP_SECTION_FILES {
        match read(rel) {
            Ok(text) => check_stop_section(&plan, rel, &text, &mut file_errors),
            Err(e) => file_errors.push(format!("{rel}: not generated ({e})")),
        }
    }

    // 3. A command in every task's Exit Criteria.
    for task in &resolved {
        let rel = format!("tasks/T{:02}-{}.md", task.number, task.slug);
        match read(&rel) {
            Ok(text) => check_task_commands(task, &rel, &text, &mut file_errors),
            Err(e) => file_errors.push(format!("{rel}: not generated ({e})")),
        }
    }

    // 4. No unrendered template markers in any generated file.
    for file in all_files(out.path()) {
        let rel = file
            .strip_prefix(out.path())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(&file).expect("generated files are UTF-8 text");
        check_markers(&rel, &text, &mut file_errors);
    }

    for e in file_errors {
        err(e);
    }
    errors
}

#[test]
fn golden_corpus_has_enough_plans() {
    assert!(
        golden_plans().len() >= 8,
        "tests/golden must hold at least eight plans"
    );
}

#[test]
fn golden_plans_generate_clean() {
    let mut report = String::new();
    let mut failed = 0;
    for path in golden_plans() {
        let errors = check_plan(&path);
        if !errors.is_empty() {
            failed += 1;
            for e in errors {
                writeln!(report, "{e}").unwrap();
            }
        }
    }
    assert!(failed == 0, "{failed} golden plan(s) failed:\n{report}");
}
