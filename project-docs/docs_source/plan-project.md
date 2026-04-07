# Project Configuration

The `[project]` section defines your project metadata.

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Project name |
| `description` | Yes | Brief description of what you're building |
| `language` | Yes | Programming language (see [Language Profiles](./language-profiles.md)) |
| `path` | Yes | Path to the project directory (output target) |
| `architecture` | No | Architecture style hint: `hexagonal`, `layered`, `modular`, `flat` |

## Example

```toml
[project]
name = "my-api"
description = "A REST API for managing inventory"
language = "rust"
path = "/home/user/projects/my-api"
architecture = "hexagonal"
```

## Language

The `language` field determines which [language profile](./language-profiles.md) is used for default build, test, and lint commands, as well as template hints like file patterns and documentation style.

Supported values: `rust`, `go`, `typescript`, `python`, `java`, `csharp`, `kotlin`, `swift`, `ruby`, `elixir`.

> **Note for .NET projects:** use `language = "csharp"` — this covers all .NET SDK project types (ASP.NET Core, console, class library, etc.) regardless of whether you write C#, F#, or VB.NET.

## Architecture

The optional `architecture` field provides hints to the generated task files about how code should be organized. This influences implementation guidance in the generated artifacts.
