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
Other targets are opt-in via `--target` or the plan's `[targets]` section:

| Target | CLI / plan field | What gets emitted |
|---|---|---|
| VSCode + Copilot (default) | `--target vscode` | `.vscode/orchestrator.prompt.md` + three siblings |
| opencode | `--target opencode` | `.opencode/agents/wiggum-*.md` (five files) |
| Claude Code | `--target claude` | `CLAUDE.md` (project memory) + `.claude/settings.json` (hooks) |
| Cursor / Windsurf / GitHub Copilot | `--target agent-rules` | `.cursorrules` + `.windsurfrules` + `.github/copilot-instructions.md` |

Run `wiggum generate plan.toml --target all` to emit everything at once. See [Targets](./targets.md) for the full reference.

## 6. Run the loop

Open your AI coding tool, load the generated orchestrator prompt as the agent
prompt, and let it work through the tasks.

- **VSCode + Copilot:** open the project, switch to agent mode, paste
  `.vscode/orchestrator.prompt.md` as the user message.
- **opencode:** open the project — the `wiggum-orchestrator` agent is
  auto-discovered from `.opencode/agents/`. Select it from the agent picker.
- **Claude Code:** open the project — `CLAUDE.md` is auto-loaded and the
  `PreCompact` hook is auto-registered. Run `claude` in the terminal.
- **Cursor / Windsurf / Copilot:** open the project — the IDE reads its
  corresponding rules file automatically.
