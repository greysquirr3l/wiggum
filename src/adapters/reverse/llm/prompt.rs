//! Prompt templates for the LLM-driven phase decomposition.
//!
//! Kept as plain `&str` builders — no jinja/tera machinery here. The output
//! spec is the critical contract; if you change [`render_user_prompt`] in a
//! way that would change the JSON shape the model emits, update
//! [`super::plan_json::LlmPlan`] to match and add a test.

/// System prompt — sets the model's role + the JSON contract it must obey.
#[must_use]
pub fn render_system_prompt() -> String {
    SYSTEM_PROMPT.to_string()
}

/// User prompt — carries the project context + asks for the JSON plan.
#[must_use]
pub fn render_user_prompt(ctx: &super::context::LlmContext) -> String {
    let hints_block = ctx.user_hints.as_deref().unwrap_or("(none provided)");

    format!(
        "PROJECT CONTEXT\n\
         ==============\n\
         \n\
         - language: {language}\n\
         - name: {name}\n\
         - description: {description}\n\
         - architecture: {architecture}\n\
         \n\
         DETECTED RULES (from the source repo)\n\
         ------------------------------------\n\
         {rules}\n\
         \n\
         TOP-LEVEL TREE (depth ≤ {depth})\n\
         --------------------------\n\
         ```\n\
         {tree}\n\
         ```\n\
         \n\
         README EXCERPT (first {readme_b} bytes)\n\
         -------------------------------------\n\
         ```\n\
         {readme}\n\
         ```\n\
         \n\
         USER HINTS\n\
         ----------\n\
         ```\n\
         {hints}\n\
         ```\n\
         \n\
         TASK\n\
         ----\n\
         Emit a `plan.toml` worth of phases + tasks for this project. Honour the\n\
         user hints above — they override whatever you would infer. Prefer\n\
         dependency order: domain/contracts first, then infrastructure, then\n\
         adapters. Aim for 3–8 phases; each phase should have 1–6 tasks. Every\n\
         task slug must be kebab-case and unique across the whole plan.\n\
         \n\
         OUTPUT\n\
         ------\n\
         Return ONLY the JSON object (no prose, no markdown fences). The shape:\n\
         \n\
         {{\n\
           \"phases\": [\n\
             {{\n\
               \"name\": \"<phase name>\",\n\
               \"tasks\": [\n\
                 {{\n\
                   \"slug\": \"<kebab-case>\",\n\
                   \"title\": \"<short>\",\n\
                   \"goal\": \"<one-sentence>\",\n\
                   \"depends_on\": [\"<slug>\", ...],\n\
                   \"hints\": [\"<hint>\", ...],\n\
                   \"gate\": \"<gate-name>\" or null\n\
                 }}\n\
               ]\n\
             }}\n\
           ]\n\
         }}\n",
        language = ctx.language,
        name = ctx.name,
        description = if ctx.description.is_empty() {
            "(none)"
        } else {
            ctx.description.as_str()
        },
        architecture = ctx.architecture.as_deref().unwrap_or("(none)"),
        rules = if ctx.detected_rules.is_empty() {
            "(none)".to_string()
        } else {
            ctx.detected_rules
                .iter()
                .map(|r| format!("- {r}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        depth = super::context::MAX_TREE_DEPTH,
        tree = ctx.tree,
        readme_b = ctx.readme_excerpt.len(),
        readme = if ctx.readme_excerpt.is_empty() {
            "(no README found)".to_string()
        } else {
            ctx.readme_excerpt.clone()
        },
        hints = hints_block,
    )
}

const SYSTEM_PROMPT: &str = "\
You are a senior software architect who decomposes a Git repository into a \
dependency-ordered implementation plan. You always respond with a single JSON \
object matching the shape below, with no commentary or markdown fences.

Schema:
{
  \"phases\": [
    {
      \"name\": string,           // phase display name, e.g. \"Domain Core\"
      \"tasks\": [
        {
          \"slug\": string,       // kebab-case, unique across the whole plan
          \"title\": string,      // short human label
          \"goal\": string,       // one sentence describing the deliverable
          \"depends_on\": [string], // slugs of tasks in earlier phases
          \"hints\": [string],    // optional implementation hints
          \"gate\": string | null // optional human-gate category name
        }
      ]
    }
  ]
}

Rules:
- Every task slug must be globally unique and kebab-case (lowercase, digits, hyphens).
- Dependencies must point at slugs that exist; no forward references.
- Do not include tasks the user explicitly said to skip.
- If the user provided phases via hints, follow that structure exactly.
- Aim for 3–8 phases and 1–6 tasks per phase.
- Prefer domain → contracts → infrastructure → adapters → tests ordering.
";
