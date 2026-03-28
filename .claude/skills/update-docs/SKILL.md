---
name: update-docs
description: Update all documentation files after code changes
---

# /update-docs

Synchronize all documentation with the current state of the codebase. You are a documentation specialist for nucleo. You never guess — you read the current source of truth before updating any doc.

## Documentation Map

| File | Purpose | Key Sections |
|------|---------|--------------|
| `CLAUDE.md` | Expert guide for Claude Code | Build, Layout, Command Tree, Architecture, Extension Guide, Env Vars, Fork Guide |
| `README.md` | Public-facing docs | Features, Architecture, Commands, Plugins, MCP, Benchmarks, Stack, Fork Guide |
| `.claude/agents/*.md` | Agent definitions | nucleo-expert (developer) |
| `.claude/skills/*.md` | Skill triggers | /add-command, /add-plugin, /add-mcp-tool, /benchmark, /update-docs |
| `benchmarks/run.sh` | Benchmark suite | COMMANDS array (~line 30) must list all benchmarkable commands |

## Update Rules by Change Type

### New Command Added

1. **`CLAUDE.md`** — Update "Project Layout" tree, "Command Tree", and "Commands" table in Architecture
2. **`README.md`** — Update commands table
3. **`benchmarks/run.sh`** — Add to COMMANDS array if the command is benchmarkable (has `--format` or is fast enough to benchmark)

### New Plugin Added

1. **`CLAUDE.md`** — Update "Project Layout" tree, "Command Tree" (under plugins), and "Shipped plugins" table
2. **`README.md`** — Update plugins section

### New MCP Tool Added

1. **`CLAUDE.md`** — Update MCP tools table in Architecture
2. **`README.md`** — Update MCP section if it lists tools

### New Template Added

1. **`CLAUDE.md`** — Update "Project Layout" tree (under templates/)

### Core Module Change

1. **`CLAUDE.md`** — Update the relevant Architecture section (error codes, formatter formats, client behavior, config keys, types)
2. **`README.md`** — Update if the change affects user-facing behavior

### New Environment Variable

1. **`CLAUDE.md`** — Add to "Environment Variables" table
2. **`README.md`** — Add to env vars section if present
3. **`.env.example`** — Add with comment

### CI/CD Change

1. **`CLAUDE.md`** — Update CI/CD section
2. **`README.md`** — Update if workflow names or triggers changed

### Dependency Change

1. **`README.md`** — Update stack section if a major dependency was added/removed

### Agent or Skill Change

1. **`CLAUDE.md`** — Update "Project Layout" tree (`.claude/` section)
2. Other agents/skills — cross-reference if they mention the changed agent/skill

## Workflow

1. **Determine change type** — run `git diff --stat` and `git diff` to understand what changed
2. **Read affected docs** — read each file that needs updating per the rules above
3. **Update** — make precise edits, matching existing style and formatting
4. **Verify** — check that:
   - File counts in layout trees match actual files (`ls src/commands/`, `ls plugins/`, etc.)
   - Command tree matches the `Command` enum in `src/main.rs`
   - MCP tools table matches `#[tool]` definitions in `src/mcp/tools.rs`
   - Plugin table matches directories in `plugins/`
   - Env var table matches actual env var usage in code
   - COMMANDS array in `benchmarks/run.sh` references valid commands

## Style Rules

- Match the existing markdown formatting exactly (heading levels, table alignment, code fence languages)
- Keep `CLAUDE.md` technical and dense — it's for Claude Code, not humans
- Keep `README.md` concise and scannable — it's for developers evaluating the project
- Never add sections that don't exist — only update existing sections or add entries to existing lists/tables
- Use the same terminology as the codebase (e.g., "presets" not "environments", "ServiceUrls" not "endpoints")
