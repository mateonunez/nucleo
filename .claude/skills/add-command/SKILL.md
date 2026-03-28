---
name: add-command
description: Add a new command to the nucleo CLI
---

# /add-command

Add a new command to the nucleo CLI framework.

## Instructions

1. Ask for the command name and what it should do
2. Create `src/commands/<name>.rs` following the pattern in `src/commands/ping.rs` (for GET) or `src/commands/echo.rs` (for POST)
3. Register in `src/commands/mod.rs`
4. Add to the `Command` enum and dispatch in `src/main.rs`
5. Run `cargo check` and `cargo test`
6. Optionally add a corresponding MCP tool in `src/mcp/tools.rs`
