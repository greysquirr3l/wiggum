# Quick Start

## 1. Create a plan interactively

```bash
wiggum init
```

This walks you through project setup — name, description, language, architecture style, phases, and tasks — and writes a `plan.toml`.

## 2. Or bootstrap from an existing project

If you already have a project directory, Wiggum can detect the language, build system, and structure to create a starter plan:

```bash
wiggum bootstrap /path/to/project
```

## 3. Validate the plan

```bash
wiggum validate plan.toml --lint
```

This checks that the dependency graph is a valid DAG and runs lint rules to catch common plan quality issues.

## 4. Preview what will be generated

```bash
wiggum generate plan.toml --dry-run --estimate-tokens
```

## 5. Generate artifacts

```bash
wiggum generate plan.toml
```

This produces the task files, `PROGRESS.md`, `IMPLEMENTATION_PLAN.md`, and the
target-specific agent prompts in your project directory.

By default, Wiggum emits VSCode + Copilot prompt files (`.vscode/*.prompt.md`).
To target opencode instead, pass `--target opencode` or add `[targets] opencode = true` to your plan. Run `wiggum generate plan.toml --target all` to emit both:

- `.vscode/orchestrator.prompt.md` (VSCode + Copilot)
- `.opencode/agents/wiggum-orchestrator.md` (opencode)

See [Targets](./targets.md) for the full reference.

## 6. Run the loop

Open your AI coding tool, load the generated orchestrator prompt as the agent
prompt, and let it work through the tasks.

- **VSCode + Copilot:** open the project, switch to agent mode, paste
  `.vscode/orchestrator.prompt.md` as the user message.
- **opencode:** open the project — the `wiggum-orchestrator` agent is
  auto-discovered from `.opencode/agents/`. Select it from the agent picker.
