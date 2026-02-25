# Getting Started

This section covers installing Wiggum and creating your first plan.

## Prerequisites

- Rust toolchain (1.85+) if building from source
- A project idea with a rough architecture in mind

## Overview

The typical workflow:

1. **Create a plan** — Either interactively with `wiggum init` or by writing a `plan.toml` by hand
2. **Generate artifacts** — Run `wiggum generate plan.toml` to produce task files, progress tracker, and orchestrator prompt
3. **Run the loop** — Use your preferred AI coding tool with the generated orchestrator prompt to execute tasks sequentially

Continue to [Installation](./installation.md) to get Wiggum on your system, or jump to [Quick Start](./quick-start.md) if you already have it installed.
