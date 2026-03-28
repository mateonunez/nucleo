---
name: add-plugin
description: Create a new plugin for the nucleo CLI
---

# /add-plugin

Create a new language-agnostic plugin for the nucleo CLI.

## Instructions

1. Ask for the plugin name, language (Node.js/Python/shell), and commands
2. Create `plugins/<name>/` with `plugin.json` manifest
3. Create the entrypoint that reads env vars and outputs JSON
4. Test with `nucleo plugins install ./plugins/<name>` and run each command
