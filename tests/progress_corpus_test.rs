//! Synthetic PROGRESS.md corpus: every `tests/progress-corpus/*.md` is parsed
//! with `parse_progress` and checked against the hand-labelled truth in
//! `tests/progress-corpus/labels.toml`.
//!
//! Adding a file is enough: drop a `*.md` into the corpus dir, add a
//! `["<file>.md"]` entry to `labels.toml` and a snapshot under `snapshots/`
//! (see `tests/progress-corpus/README.md`). A file with no labels entry fails.
//!
//! Labels are the truth. Where today's parser or `wiggum retro` disagrees,
//! the label is NOT bent: the field is listed under `known_defects` with the
//! value seen today (`today`). The test then asserts the defect is still
//! there (actual differs from truth and equals the recorded value), so a fix
//! shows up as a failure telling you to delete the `known_defects` entry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use regex::Regex;
use toml::{Table, Value};

use wiggum::domain::report::{TaskStatus, parse_progress};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/progress-corpus")
}

/// Corpus files (`*.md` except the README), sorted by name.
fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("corpus dir exists")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter(|p| p.file_name().is_some_and(|n| n != "README.md"))
        .collect();
    files.sort();
    files
}

fn file_name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

fn labels() -> Table {
    let text = std::fs::read_to_string(corpus_dir().join("labels.toml")).expect("labels.toml");
    text.parse::<Table>().expect("labels.toml is valid TOML")
}

fn int(n: usize) -> Value {
    Value::Integer(i64::try_from(n).unwrap())
}

fn count_status(content: &str, status: TaskStatus) -> usize {
    parse_progress(content)
        .tasks
        .iter()
        .filter(|t| t.status == status)
        .count()
}

/// Verdict headings (`## T25 re-evaluation FAIL (date, about 8.3/10) ...`) in
/// file order, per task, as `FAIL:8.3` strings. Tolerates wording variants:
/// any `## T<n>` heading whose text before the first ` (` names an
/// evaluation or review and contains PASS or FAIL.
fn verdict_headings(content: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in content.lines() {
        let Some(rest) = line.strip_prefix("## T") else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let head = line.split(" (").next().unwrap_or(line);
        let lower = head.to_lowercase();
        if !(lower.contains("evaluat") || lower.contains("review")) {
            continue;
        }
        let verdict = if head.contains("FAIL") {
            "FAIL"
        } else if head.contains("PASS") {
            "PASS"
        } else {
            continue;
        };
        let score = line.find("/10").map_or(String::new(), |end| {
            let before = &line[..end];
            let rev: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            rev.chars().rev().collect()
        });
        out.entry(format!("T{digits}"))
            .or_default()
            .push(format!("{verdict}:{score}"));
    }
    out
}

/// Explicit `retry <digit>` mentions outside `## ` headings (task notes and
/// learnings that record a retry without a verdict heading).
fn explicit_retry_mentions(content: &str) -> usize {
    let re = Regex::new(r"(?i)retry \d").unwrap();
    content
        .lines()
        .filter(|l| !l.starts_with("## "))
        .map(|l| re.find_iter(l).count())
        .sum()
}

/// `(tasks, retries, gates)` from a snapshot's first line.
fn snapshot_header(snapshot: &str) -> (usize, usize, usize) {
    let first = snapshot.lines().next().expect("snapshot is not empty");
    let re = Regex::new(r"\((\d+) tasks, (\d+) retries, (\d+) gates\)").unwrap();
    let unrecognised = format!("snapshot header not recognised: {first}");
    let c = re.captures(first).expect(&unrecognised);
    (
        c[1].parse().unwrap(),
        c[2].parse().unwrap(),
        c[3].parse().unwrap(),
    )
}

struct Checker<'a> {
    file: &'a str,
    entry: &'a Table,
    fails: Vec<String>,
}

impl Checker<'_> {
    fn truth(&self, field: &str) -> Value {
        let missing = format!("{}: labels entry has no `{field}`", self.file);
        self.entry.get(field).expect(&missing).clone()
    }

    /// Compare `actual` with the label `truth_field`. When the field is a
    /// known defect, assert the defect is still present and pinned.
    fn check(&mut self, field: &str, actual: &Value, truth: &Value) {
        let defect = self
            .entry
            .get("known_defects")
            .and_then(Value::as_table)
            .and_then(|t| t.get(field));
        match defect {
            None => {
                if actual != truth {
                    self.fails.push(format!(
                        "{}: field `{field}`: label (truth) is {truth}, parser/retro gave {actual}",
                        self.file
                    ));
                }
            }
            Some(reason) => {
                if actual == truth {
                    self.fails.push(format!(
                        "{}: field `{field}` is listed in known_defects ({reason}) but now matches the truth {truth}; remove it from known_defects and today",
                        self.file
                    ));
                }
                let today = self
                    .entry
                    .get("today")
                    .and_then(Value::as_table)
                    .and_then(|t| t.get(field));
                if today != Some(actual) {
                    self.fails.push(format!(
                        "{}: field `{field}` known defect value changed: recorded today = {today:?}, now {actual}",
                        self.file
                    ));
                }
            }
        }
    }

    fn check_label(&mut self, field: &str, actual: &Value) {
        let truth = self.truth(field);
        self.check(field, actual, &truth);
    }
}

/// Verdict sequences, retry counts and gate ids: label consistency plus a
/// cross-check of the labels against the file text itself.
fn check_retries_and_verdicts(c: &mut Checker<'_>, content: &str, task_ids: &[String]) {
    let name = c.file;
    let entry = c.entry;
    let verdicts_label: BTreeMap<String, Vec<String>> = entry["verdicts"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            let seq = v
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            (k.clone(), seq)
        })
        .collect();
    let fail_count: usize = verdicts_label
        .values()
        .flatten()
        .filter(|v| v.starts_with("FAIL"))
        .count();
    let explicit = usize::try_from(entry["explicit_retries"].as_integer().unwrap()).unwrap();
    let real = usize::try_from(entry["real_retries"].as_integer().unwrap()).unwrap();
    if fail_count + explicit != real {
        c.fails.push(format!(
            "{name}: labels are inconsistent: real_retries {real} != FAIL verdicts {fail_count} + explicit_retries {explicit}"
        ));
    }
    let found = verdict_headings(content);
    if found != verdicts_label {
        c.fails.push(format!(
            "{name}: verdict headings in the file {found:?} differ from labelled verdicts {verdicts_label:?}"
        ));
    }
    let mentions = explicit_retry_mentions(content);
    if mentions != explicit {
        c.fails.push(format!(
            "{name}: file has {mentions} explicit `retry N` mentions outside headings, label explicit_retries is {explicit}"
        ));
    }
    for g in entry["gate_tasks"].as_array().unwrap() {
        let id = g.as_str().unwrap();
        if !task_ids.iter().any(|t| t == id) {
            c.fails.push(format!(
                "{name}: gate_tasks names `{id}` which is not a task row"
            ));
        }
    }
}

/// The retro snapshot header must equal what the labels recorded, and the
/// retro-derived retries and gates are compared with the truth.
fn check_snapshot(c: &mut Checker<'_>, task_total: usize) {
    let name = c.file;
    let entry = c.entry;
    let snap_path = corpus_dir()
        .join("snapshots")
        .join(name.replace(".md", ".retro.txt"));
    let snap = match std::fs::read_to_string(&snap_path) {
        Ok(s) => s,
        Err(e) => {
            c.fails.push(format!(
                "{name}: missing snapshot {}: {e}",
                snap_path.display()
            ));
            return;
        }
    };
    let (tasks, retries, gates) = snapshot_header(&snap);
    if tasks != task_total {
        c.fails.push(format!(
            "{name}: snapshot says {tasks} tasks but parse_progress finds {task_total}; regenerate snapshots"
        ));
    }
    let rt = entry["retro_today"].as_table().unwrap();
    let recorded = (
        usize::try_from(rt["tasks"].as_integer().unwrap()).unwrap(),
        usize::try_from(rt["retries"].as_integer().unwrap()).unwrap(),
        usize::try_from(rt["gates"].as_integer().unwrap()).unwrap(),
    );
    if recorded != (tasks, retries, gates) {
        c.fails.push(format!(
            "{name}: snapshot header {:?} differs from labels retro_today {recorded:?}",
            (tasks, retries, gates)
        ));
    }
    c.check("retro_retries", &int(retries), &entry["real_retries"]);
    let gate_truth = int(entry["gate_tasks"].as_array().unwrap().len());
    c.check("retro_gates", &int(gates), &gate_truth);
}

/// Parser-derived fields: project name, task and status counts, phases.
fn check_parse(c: &mut Checker<'_>, content: &str) {
    let summary = parse_progress(content);
    c.check_label("project_name", &Value::String(summary.project_name.clone()));
    c.check_label("tasks", &int(summary.tasks.len()));
    c.check_label(
        "completed",
        &int(count_status(content, TaskStatus::Completed)),
    );
    c.check_label(
        "in_progress",
        &int(count_status(content, TaskStatus::InProgress)),
    );
    c.check_label("blocked", &int(count_status(content, TaskStatus::Blocked)));
    c.check_label(
        "not_started",
        &int(count_status(content, TaskStatus::NotStarted)),
    );
    c.check_label("phases", &int(summary.phases.len()));
    let known_phase_defect = c
        .entry
        .get("known_defects")
        .and_then(Value::as_table)
        .is_some_and(|t| t.contains_key("phases"));
    if !known_phase_defect {
        let names = Value::Array(
            summary
                .phases
                .iter()
                .map(|p| Value::String(p.name.clone()))
                .collect(),
        );
        c.check_label("phase_names", &names);
    }
}

#[test]
fn corpus_matches_labels() {
    let labels = labels();
    let files = corpus_files();
    assert!(files.len() >= 8, "corpus must keep at least 8 files");
    let mut fails: Vec<String> = Vec::new();

    for name in labels.keys() {
        if !files.iter().any(|f| file_name(f) == *name) {
            fails.push(format!(
                "labels.toml has an entry for `{name}` but no such corpus file"
            ));
        }
    }

    for path in &files {
        let name = file_name(path);
        let Some(entry) = labels.get(&name).and_then(Value::as_table) else {
            fails.push(format!(
                "{name}: corpus file has no entry in tests/progress-corpus/labels.toml (add a [\"{name}\"] table with the hand-labelled truth)"
            ));
            continue;
        };
        let content = std::fs::read_to_string(path).unwrap();
        let summary = parse_progress(&content);
        let task_ids: Vec<String> = summary.tasks.iter().map(|t| t.id.clone()).collect();
        let mut c = Checker {
            file: &name,
            entry,
            fails: Vec::new(),
        };
        check_parse(&mut c, &content);
        check_retries_and_verdicts(&mut c, &content, &task_ids);
        check_snapshot(&mut c, summary.tasks.len());
        fails.extend(c.fails);
    }

    assert!(
        fails.is_empty(),
        "progress corpus mismatches:\n  {}",
        fails.join("\n  ")
    );
}

#[test]
fn every_corpus_file_has_exactly_one_snapshot() {
    let files = corpus_files();
    let snaps: Vec<String> = std::fs::read_dir(corpus_dir().join("snapshots"))
        .expect("snapshots dir")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".retro.txt"))
        .collect();
    assert_eq!(
        snaps.len(),
        files.len(),
        "snapshots {snaps:?} vs corpus files {:?}",
        files.iter().map(|f| file_name(f)).collect::<Vec<_>>()
    );
}

#[test]
fn corpus_has_no_real_world_leaks() {
    let url = Regex::new(r"https?://").unwrap();
    let hexrun = Regex::new(r"[0-9a-fA-F]{8,}").unwrap();
    let email = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+\.[A-Za-z]{2,}").unwrap();
    let ticked_hash = Regex::new(r"`([0-9a-f]{7})`").unwrap();

    let mut targets: Vec<PathBuf> = corpus_files();
    targets.push(corpus_dir().join("labels.toml"));
    for e in std::fs::read_dir(corpus_dir().join("snapshots")).unwrap() {
        targets.push(e.unwrap().path());
    }

    let mut problems = String::new();
    for path in targets {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = file_name(&path);
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            if url.is_match(line) {
                writeln!(problems, "{name}:{n}: contains a URL").unwrap();
            }
            for m in hexrun.find_iter(line) {
                let s = m.as_str();
                if s.len() >= 40 || s.chars().any(|c| c.is_ascii_digit()) {
                    writeln!(problems, "{name}:{n}: hash-like hex run of {} chars (only 7-hex fake hashes in backticks are allowed)", s.len()).unwrap();
                }
            }
            if line.contains("/Users/") || line.contains("/home/") {
                writeln!(problems, "{name}:{n}: contains an absolute home path").unwrap();
            }
            if email.is_match(line) {
                writeln!(problems, "{name}:{n}: contains an email address").unwrap();
            }
        }
    }
    assert!(problems.is_empty(), "leak guard failed:\n{problems}");

    // The allowed hash shape is exactly seven hex digits in backticks.
    let sample: usize = corpus_files()
        .iter()
        .map(|p| {
            ticked_hash
                .find_iter(&std::fs::read_to_string(p).unwrap())
                .count()
        })
        .sum();
    assert!(sample > 0, "expected the corpus to carry fake 7-hex hashes");
}

#[test]
fn verdict_extractor_handles_wording_variants() {
    let md = "\
## T01 evaluator FAIL (2027-01-01, about 6.8/10) on `abc1234` — retry 1 of 2
## T01 re-evaluation FAIL (2027-01-02, about 8.3/10) — retry 2 of 2 (LAST)
## T01 final re-evaluation PASS (2027-01-03, 9/10)
## T02 security evaluation: conditional PASS (2027-01-03, about 9.1/10)
## T03 retry 1: what now exists
## T03 learnings
";
    let got = verdict_headings(md);
    assert_eq!(got["T01"], ["FAIL:6.8", "FAIL:8.3", "PASS:9"]);
    assert_eq!(got["T02"], ["PASS:9.1"]);
    assert!(!got.contains_key("T03"));
}
