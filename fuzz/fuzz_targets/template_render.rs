#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(plan) = wiggum::domain::plan::Plan::from_toml(input) else {
        return;
    };
    let tasks = plan.resolve_tasks().unwrap_or_default();

    let _ = wiggum::generation::agents_md::render(&plan);
    let _ = wiggum::generation::planner::render(&plan);
    let _ = wiggum::generation::background_auditor::render(&plan);

    if let Some(task) = tasks.first() {
        let _ = wiggum::generation::task::render(&plan, task);
    }

    let _ = wiggum::generation::orchestrator::render(&plan, &tasks);
    let _ = wiggum::generation::progress::render(&plan, &tasks);
    let _ = wiggum::generation::plan_doc::render(&plan, &tasks);
    let _ = wiggum::generation::features::render(&plan, &tasks);
    let _ = wiggum::generation::evaluator::render(&plan, &tasks);
});
