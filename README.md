# nucleo

> The nucleus of your next CLI.

A reusable Rust CLI framework with batteries included: plugin system, template scaffolding, MCP server, output formatting, layered config, and benchmarks. Fork it, change 4 constants, build your domain.

## Quick Start

```sh
git clone <your-fork>
cd nucleo

# Customize identity (src/consts.rs)
# APP_NAME, APP_DIR, APP_PREFIX, APP_BIN

cargo build --release
./target/release/nucleo --help
```

## Features

- **9 native commands** — auth, config, status, ping, echo, completions, plugins, mcp, setup
- **6 output formats** — JSON, table, YAML, CSV, IDs, Slack mrkdwn
- **Plugin system** — language-agnostic (Python, TypeScript, Rust, Go, anything) via subprocess protocol
- **Scaffold plugin** — create projects, layouts, and components from templates
- **MCP server** — Claude Desktop integration out of the box via `nucleo mcp`
- **Layered config** (JSON) — env vars > files > defaults, with user-defined environment presets
- **OAuth2 PKCE** — Authorization Code flow with PKCE for APIs like Spotify, GitHub, Google
- **HTTP client** — retry on 429, automatic token refresh (basic + OAuth2), 401 retry
- **Error system** — typed errors with distinct exit codes (1/2/3/5) and JSON output
- **Benchmarks** — token consumption and execution speed measurement
- **CI/CD** — GitHub Actions for quality (check, test, clippy, fmt) and cross-platform release
- **Claude Code integration** — agents, skills, and CLAUDE.md for AI-assisted development
- **Shell completions** — bash, zsh, fish, powershell, elvish

## Architecture

```
src/
├── main.rs          # Clap derive tree + async dispatch
├── consts.rs        # 4 constants — the only file to change when forking
├── error.rs         # CliError enum with exit codes
├── formatter.rs     # 6 output formats
├── client.rs        # HTTP client with retry + auth
├── config.rs        # Layered config (JSON) with HashMap-based service URLs
├── oauth2.rs        # OAuth2 Authorization Code + PKCE
├── types/           # Auth, OAuth2 config, project context, pagination
├── commands/        # All CLI commands
└── mcp/             # MCP server for AI assistant integration
```

## How to Fork

1. Edit `src/consts.rs`:
```rust
pub const APP_NAME: &str = "mycli";
pub const APP_DIR: &str = "mycli";
pub const APP_PREFIX: &str = "MYCLI";
pub const APP_BIN: &str = "mycli";
```

2. Update `Cargo.toml`:
```toml
[package]
name = "mycli"

[[bin]]
name = "mycli"
```

3. Copy `config.json` to `~/.config/mycli/config.json` and define your service URLs:
```json
{
  "urls": {},
  "active_env": "dev",
  "presets": {
    "dev": {
      "auth": "https://auth.dev.example.com/api/v2",
      "api": "https://api.dev.example.com/api/v1"
    }
  }
}
```

For OAuth2 APIs (Spotify, GitHub, etc.), use the structured preset format:
```json
{
  "presets": {
    "dev": {
      "urls": { "api": "https://api.spotify.com/v1" },
      "auth_method": "oauth2",
      "oauth2": {
        "client_id": "your-client-id",
        "authorize_url": "https://accounts.spotify.com/authorize",
        "token_url": "https://accounts.spotify.com/api/token",
        "scopes": ["user-read-playback-state"]
      }
    }
  }
}
```

4. Replace example commands (`ping`, `echo`) with your domain commands
5. `cargo build --release`

## Commands

| Command | Description |
|---------|-------------|
| `auth login\|logout\|token` | Manage authentication (basic or OAuth2 PKCE) |
| `config show\|env\|set` | View and modify configuration |
| `status` | System, auth, and config overview |
| `ping` | HTTP GET example (test connectivity) |
| `echo` | Authenticated HTTP POST example |
| `plugins` | Install, remove, upgrade, and run plugins |
| `mcp` | Start MCP server for Claude Desktop |
| `setup` | Interactive setup wizard |
| `completions` | Generate shell completions |

## Plugins

Plugins are language-agnostic extensions. Any executable with a `plugin.json` manifest works:

```sh
nucleo plugins install ./plugins/hello       # install
nucleo plugins hello greet                   # run
nucleo plugins list                          # list installed
nucleo plugins remove hello                  # uninstall
```

**Shipped plugins:**

| Plugin | Language | Purpose |
|--------|----------|---------|
| `hello` | TypeScript | Example plugin demonstrating the protocol |
| `scaffold` | Node.js | Template scaffolding for new projects |

```sh
nucleo plugins install ./plugins/scaffold
nucleo plugins scaffold list                            # list templates
nucleo plugins scaffold create my-app hello-api         # scaffold a project
```

## MCP Server

Connect nucleo to Claude Desktop:

```json
{
  "mcpServers": {
    "nucleo": {
      "command": "nucleo",
      "args": ["mcp"]
    }
  }
}
```

## Benchmarks

```sh
./benchmarks/run.sh              # full suite
./benchmarks/run.sh --quick      # smoke test
./benchmarks/run.sh --formats    # compare output formats
./benchmarks/run.sh --json       # raw JSON
```

## Stack

- **Rust** (edition 2024, rust-version 1.85)
- **clap** 4.6 (derive macros)
- **tokio** 1.50 (async runtime)
- **reqwest** 0.12 (HTTP client)
- **rmcp** 1.3 (MCP server)
- **serde** / **serde_json** (serialization)
- **sha2** 0.10 / **rand** 0.9 (OAuth2 PKCE)
- **thiserror** 2 / **anyhow** (error handling)

## License

[MIT](LICENSE)
