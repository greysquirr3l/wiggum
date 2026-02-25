use std::path::Path;

use dialoguer::{Confirm, Input, MultiSelect};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};

/// Interactively add a task to an existing plan TOML file.
///
/// # Errors
///
/// Returns an error if the plan file cannot be read or parsed, if interactive
/// prompts fail, or if the file cannot be written back.
pub fn run_add_task(plan_path: &Path) -> Result<()> {
    let content = std::fs::read_to_string(plan_path)?;

    // Parse with serde to extract existing slugs for dependency selection
    let plan = Plan::from_toml(&content)?;
    let existing_slugs: Vec<String> = plan
        .phases
        .iter()
        .flat_map(|p| p.tasks.iter().map(|t| t.slug.clone()))
        .collect();

    let phase_names: Vec<String> = plan.phases.iter().map(|p| p.name.clone()).collect();

    // Parse with toml_edit to preserve formatting
    let mut doc: DocumentMut = content
        .parse()
        .map_err(|e: toml_edit::TomlError| WiggumError::PlanParse(e.to_string()))?;

    // Ask which phase
    let phase_idx = dialoguer::Select::new()
        .with_prompt("Add task to which phase?")
        .items(&phase_names)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    // Collect task info
    let slug: String = Input::new()
        .with_prompt("Task slug (kebab-case)")
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    if existing_slugs.contains(&slug) {
        return Err(WiggumError::DuplicateSlug(slug));
    }

    let title: String = Input::new()
        .with_prompt("Task title")
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let goal: String = Input::new()
        .with_prompt("Task goal")
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let depends_on: Vec<String> = if existing_slugs.is_empty() {
        Vec::new()
    } else {
        let selected = MultiSelect::new()
            .with_prompt("Dependencies (space to toggle, enter to confirm)")
            .items(&existing_slugs)
            .interact()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;
        selected
            .into_iter()
            .filter_map(|i| existing_slugs.get(i).cloned())
            .collect()
    };

    let add_hints = Confirm::new()
        .with_prompt("Add implementation hints?")
        .default(false)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let hints: Vec<String> = if add_hints {
        collect_lines("  Hint (empty to finish)")?
    } else {
        Vec::new()
    };

    let add_test_hints = Confirm::new()
        .with_prompt("Add test hints?")
        .default(false)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let test_hints: Vec<String> = if add_test_hints {
        collect_lines("  Test hint (empty to finish)")?
    } else {
        Vec::new()
    };

    let task_table = build_task_table(&slug, &title, &goal, &depends_on, &hints, &test_hints);

    // Find the correct phase in the TOML document and append the task
    let phases = doc
        .get_mut("phases")
        .and_then(|p| p.as_array_of_tables_mut())
        .ok_or_else(|| WiggumError::PlanParse("no [[phases]] in plan".to_string()))?;

    let phase = phases
        .get_mut(phase_idx)
        .ok_or_else(|| WiggumError::Validation(format!("phase index {phase_idx} not found")))?;

    let tasks = phase
        .get_mut("tasks")
        .and_then(|t| t.as_array_of_tables_mut())
        .ok_or_else(|| {
            WiggumError::PlanParse("no [[phases.tasks]] in selected phase".to_string())
        })?;

    tasks.push(task_table);

    // Write back
    std::fs::write(plan_path, doc.to_string())?;

    let phase_name = phase_names.get(phase_idx).map_or("unknown", String::as_str);
    println!("✅ Added task \"{slug}\" to phase \"{phase_name}\"");

    Ok(())
}

fn build_task_table(
    slug: &str,
    title: &str,
    goal: &str,
    depends_on: &[String],
    hints: &[String],
    test_hints: &[String],
) -> Table {
    let mut t = Table::new();
    t.insert("slug", Item::Value(Value::from(slug)));
    t.insert("title", Item::Value(Value::from(title)));
    t.insert("goal", Item::Value(Value::from(goal)));

    let mut insert_array = |key: &str, items: &[String]| {
        if !items.is_empty() {
            let mut arr = Array::new();
            for item in items {
                arr.push(item.as_str());
            }
            t.insert(key, Item::Value(Value::Array(arr)));
        }
    };
    insert_array("depends_on", depends_on);
    insert_array("hints", hints);
    insert_array("test_hints", test_hints);
    t
}

fn collect_lines(prompt: &str) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    loop {
        let line: String = Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;
        if line.is_empty() {
            break;
        }
        lines.push(line);
    }
    Ok(lines)
}
