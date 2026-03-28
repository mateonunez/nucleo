---
name: add-mcp-tool
description: Add a new MCP tool to the nucleo server
---

# /add-mcp-tool

Add a new MCP tool to the nucleo MCP server for Claude Desktop integration.

## Instructions

1. Ask for the tool name and what CLI command it wraps
2. Add a params struct in `src/mcp/tools.rs`
3. Add a `#[tool]` method to the `NucleoServer` impl using the `Parameters<T>` extractor
4. The tool should call `self.run()` or `self.run_owned()` with the CLI args
5. Run `cargo check` to verify
