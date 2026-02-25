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

This produces the task files, `PROGRESS.md`, `IMPLEMENTATION_PLAN.md`, and `orchestrator.prompt.md` in your project directory.

## 6. Run the loop

Open your AI coding tool, load the generated `orchestrator.prompt.md` as the agent prompt, and let it work through the tasks.
