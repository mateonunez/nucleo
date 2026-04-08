# nucleo — Claude Expert Guide

Reusable Rust CLI framework — the nucleus of your next CLI. Fork it, change 4 constants, build your domain.

## Build & Run

```sh
cargo install --path .           # install to PATH as `nucleo`
cargo build                      # debug build
cargo build --release            # release build
cargo test                       # run all unit tests
cargo run -- <args>              # run without installing
```

## Project Layout

```
src/
├── main.rs              CLI root: clap derive tree, async dispatch, shell completions
├── consts.rs            APP_NAME, APP_DIR, APP_PREFIX, APP_BIN — change these to fork
├── error.rs             CliError enum, exit codes (1/2/3/5), JSON error output
├── formatter.rs         OutputFormat: Json/Table/Yaml/Csv/Ids/Slack
├── client.rs            reqwest client, retry on 429, token auto-refresh (basic + OAuth2), 401 retry
├── config.rs            Layered config (JSON), PresetConfig (legacy flat / structured), env presets
├── oauth2.rs            OAuth2 Authorization Code + PKCE: generate, callback server, token exchange
├── types/
│   ├── mod.rs
│   ├── auth.rs          Credentials (+ auth_method, scopes), JwtPayload, decode_jwt_payload
│   ├── oauth2.rs        OAuth2Config (client_id, authorize_url, token_url, scopes)
│   ├── project.rs       ProjectContext
│   └── common.rs        PaginatedResponse<T>, PaginationParams
├── commands/
│   ├── mod.rs
│   ├── auth.rs          login (basic + OAuth2), logout, token
│   ├── config_cmd.rs    show, env (list/use), set
│   ├── status.rs        System + auth + config status overview
│   ├── ping.rs          Example HTTP GET command (demonstrates client pattern)
│   ├── echo.rs          Example authenticated HTTP POST command
│   ├── plugins.rs       Plugin lifecycle: list, install, remove, upgrade, info, execute
│   ├── setup.rs         5-step interactive setup wizard (auto-detects OAuth2)
│   └── mcp_cmd.rs       MCP server launcher (delegates to mcp::start)
└── mcp/
    ├── mod.rs           NucleoServer + ServerHandler impl
    ├── tools.rs         3 example MCP tools (status, ping, plugins_list)
    └── executor.rs      Subprocess execution with timeout
plugins/
├── hello/               TypeScript example plugin (demonstrates plugin protocol)
└── scaffold/            Node.js template scaffolding plugin
templates/
└── hello-api/           Node.js starter template (demonstrates template engine)
benchmarks/
├── run.sh               Benchmark runner (token consumption + execution speed)
└── results/.gitignore   Results directory (gitignored)
docs/
├── README.md            Documentation index
└── guides/
    └── create-a-new-cli.md  Step-by-step guide: fork nucleo, build a CLI for any API
config.json              Default configuration (copy to ~/.config/nucleo/config.json)
.env.example             Environment variable template
.claude/
├── agents/              Agent definitions (nucleo-expert)
└── skills/              Skills (/add-command, /add-plugin, /add-mcp-tool, /benchmark, /create-cli, /update-docs)
.github/workflows/
├── ci.yml               CI: check, test, clippy, fmt
└── release.yml          Release: cross-platform builds + GitHub Release
```

## Command Tree

```
nucleo
├── auth          login [--username] [--password] [--oauth2] [--no-browser] | logout | token
├── config        show | env (list | use <preset>) | set <key> <value>
├── status        [--format text|json|yaml|csv]
├── ping          [--service <name> | --url <url>] [--format]
├── echo          [--data <json>] [--url <url>] [--format]
├── completions   <shell>   (bash | zsh | fish | powershell | elvish)
├── plugins
│   ├── list [--format]
│   ├── install <source>
│   ├── remove <name>
│   ├── upgrade [<name>]
│   ├── info <name> [--format]
│   ├── scaffold list | create <name> <template> [--dry-run]
│   ├── hello greet [name] | status
│   └── <name> [subcommand] [args...]
├── mcp           (starts MCP server on stdio)
└── setup         [--username] [--password] [--env] [--claude-desktop] [--claude-desktop-only] [--check]
```

## Architecture Reference

### Constants (`consts.rs`)

The single source of truth for customization. Change these 4 values when forking:

```rust
pub const APP_NAME: &str = "nucleo";    // display name
pub const APP_DIR: &str = "nucleo";     // config directory name (~/.config/<APP_DIR>/)
pub const APP_PREFIX: &str = "NUCLEO";  // env var prefix (NUCLEO_TOKEN, NUCLEO_AUTH_URL, etc.)
pub const APP_BIN: &str = "nucleo";     // binary name for error messages and subprocess resolution
```

### Error System (`error.rs`)

All functions return `Result<_, CliError>`. Four variants with distinct exit codes:

| Variant | Exit Code | When to use |
|---------|:---------:|-------------|
| `Api { code, message, reason }` | 1 | API returned non-2xx (except 401) |
| `Auth(String)` | 2 | 401 response or missing/invalid token |
| `Validation(String)` | 3 | Bad CLI args or user input |
| `Other(anyhow::Error)` | 5 | Everything else |

`print_error_json()` emits structured JSON to stdout. Never `panic!` or `unwrap` in command logic.

### HTTP Client (`client.rs`)

- `build_client()` — reqwest client with `nucleo/<version>` User-Agent
- `send_with_retry(|| req_builder)` — retries up to 3x on HTTP 429 with `Retry-After` / exponential backoff
- `send_authenticated(&http, |token| req_builder)` — wraps `send_with_retry` with credential loading, proactive token refresh (120s before expiry), and one 401-triggered refresh-and-retry
- `handle_api_response(resp)` — returns `serde_json::Value` on 2xx; `CliError::Auth` on 401; `CliError::Api` otherwise

Token refresh branches on `Credentials.auth_method`:
- `"basic"` — POST to `{auth_url}/refresh` with query param + Bearer header (existing flow)
- `"oauth2"` — POST to `{token_url}` with `grant_type=refresh_token` via `oauth2::refresh_oauth2()`

### Output Formatting (`formatter.rs`)

Six formats: `json` (default), `table`, `yaml`, `csv`, `ids`, `slack`.

- `OutputFormat::from_str(s)` — infallible, falls back to Json
- `OutputFormat::parse(s)` — fallible, returns validation error
- Table renderer auto-detects array key, caps cells at 60 chars

### Config System (`config.rs`)

**Config directory:** `~/.config/nucleo/`

**Config format:** JSON (`config.json`)

**Files:**

| File | Purpose |
|------|---------|
| `credentials.json` | `Credentials` (access_token, refresh_token, expires, permissions, auth_method, scopes) |
| `context.json` | `ProjectContext` (project_id, env_id, api_key, stage) |
| `config.json` | `AppConfig` (urls, active_env, presets, plugins) |
| `plugins/` | Installed plugins directory |

**ServiceUrls** — `HashMap<String, String>` (not a fixed struct).

**PresetConfig** — `#[serde(untagged)]` enum supporting two formats:
- **Legacy (flat):** `{ "auth": "https://...", "api": "https://..." }` — treated as basic auth
- **Structured:** `{ "urls": { ... }, "auth_method": "oauth2", "oauth2": { ... } }`

**Basic auth preset (legacy format):**
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

**OAuth2 preset (structured format):**
```json
{
  "urls": {},
  "active_env": "dev",
  "presets": {
    "dev": {
      "urls": { "api": "https://api.spotify.com/v1" },
      "auth_method": "oauth2",
      "oauth2": {
        "client_id": "your-client-id",
        "authorize_url": "https://accounts.spotify.com/authorize",
        "token_url": "https://accounts.spotify.com/api/token",
        "scopes": ["user-read-playback-state", "playlist-read-private"]
      }
    }
  }
}
```

**Env var overrides:** `{APP_PREFIX}_{KEY}_URL` takes precedence (e.g. `NUCLEO_AUTH_URL`).

**Token resolution:** `{APP_PREFIX}_TOKEN` env var → `credentials.json`

**Key functions:**
- `require_url(&urls, "auth")` — returns URL or `CliError::Validation`
- `load_service_urls()` — merges URLs from active preset, top-level config, and env var overrides (preset < top-level < env vars)
- `env_preset(name)` / `env_preset_names()` — reads from config.json presets
- `load_active_preset()` — resolves active preset as `EnvironmentPreset` (falls back to first preset if `active_env` is empty)
- `load_oauth2_config()` — extracts `OAuth2Config` from active preset
- `load_credentials()` / `save_credentials()` / `remove_credentials()`
- `load_context()` / `save_context()`
- `config_dir()` / `plugins_dir()`

### Types

- `Credentials` — access_token, refresh_token, expires (Unix ts), permissions, auth_method ("basic"/"oauth2"), scopes
  - `decode_payload()` — decodes JWT (no signature verify)
  - `expires_soon(margin_secs)` / `is_expired()` / `is_admin()`
- `JwtPayload` — sub, exp, email, name, username, permissions (`Option<Vec<String>>`)
- `OAuth2Config` — client_id, authorize_url, token_url, scopes, client_secret (optional), redirect_path
- `ProjectContext` — project_id, env_id, api_key, stage
- `PaginatedResponse<T>` — generic paginated API response
- `PaginationParams` — page_size/page_token with `.apply(req_builder)`

### OAuth2 (`oauth2.rs`)

Authorization Code flow with PKCE for public CLI clients. No `client_secret` needed.

**Functions:**
- `generate_pkce()` — random verifier + SHA-256 challenge (base64url)
- `build_authorize_url(config, pkce, state, redirect_uri)` — full authorization URL
- `start_callback_server(path)` — local HTTP server on port 8888 (falls back to random if busy), parses `?code=&state=`
- `exchange_code(http, config, code, verifier, redirect_uri)` — POST to token endpoint
- `refresh_oauth2(http, config, refresh_token)` — POST with `grant_type=refresh_token`
- `open_browser(url)` — platform-specific (macOS/Linux/Windows)
- `generate_state()` — random CSRF state string

**OAuth2 login flow:**
1. Load `OAuth2Config` from active preset
2. Generate PKCE challenge + state
3. Start callback server on `127.0.0.1:0` (random port)
4. Open browser (or print URL with `--no-browser`)
5. Await callback (120s timeout), validate state
6. Exchange code for tokens
7. Save credentials with `auth_method: "oauth2"`

### Plugin System (`commands/plugins.rs`)

Language-agnostic plugins via subprocess execution. Plugins are directories with a `plugin.json` manifest.

**Shipped plugins:**

| Plugin | Language | Commands | Purpose |
|--------|----------|----------|---------|
| `hello` | TypeScript | `greet`, `status` | Example: demonstrates the plugin protocol |
| `scaffold` | Node.js | `list`, `create` | Template scaffolding for new projects |

**Manifest schema (`plugin.json`):**
```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "What it does",
  "author": "you",
  "license": "MIT",
  "engine": {
    "command": "python3",
    "args": ["main.py"]
  },
  "commands": {
    "greet": { "description": "Say hello" }
  },
  "cli_version": ">=0.1.0"
}
```

**Env vars injected into plugins:**

| Variable | Source |
|----------|--------|
| `{PREFIX}_TOKEN` | Best-effort token load |
| `{PREFIX}_{KEY}_URL` | All service URLs from config |
| `{PREFIX}_PROJECT_ID`, `_ENV_ID`, `_API_KEY`, `_STAGE` | Project context |
| `{PREFIX}_PLUGIN_DIR` | Plugin's directory (absolute) |
| `{PREFIX}_PLUGIN_NAME` | Plugin name from manifest |
| `CLI_ENV_PREFIX` | The prefix itself (so plugins can be prefix-aware) |

Timeout: 120 seconds. Exit codes: 0/1/2/3/5.

### MCP Server (`mcp/`)

`nucleo mcp` starts an MCP server over stdio for Claude Desktop integration. Uses `rmcp` 1.3 with `Parameters<T>` extractor pattern.

**3 example tools:**

| Tool | Maps to |
|------|---------|
| `nucleo_status` | `nucleo status --format json` |
| `nucleo_ping` | `nucleo ping --format json [--service <name>]` |
| `nucleo_plugins_list` | `nucleo plugins list --format json` |

**Claude Desktop config:**
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

### Scaffold Plugin (`plugins/scaffold/`)

Template engine implemented as a plugin. Scaffolds new projects from template directories.

```sh
nucleo plugins scaffold list                              # list templates
nucleo plugins scaffold create my-app hello-api           # create project
nucleo plugins scaffold create my-app hello-api --dry-run # preview
```

Templates use `{{placeholder}}` markers. Replacements: `project_name` + all configured service URLs as `{key}_url`.

**Template directory resolution:** `{APP_PREFIX}_TEMPLATES_DIR` env var → `./templates/` → `<plugin_dir>/../../templates/`

### Benchmarks (`benchmarks/run.sh`)

```sh
./benchmarks/run.sh              # full suite, markdown report
./benchmarks/run.sh --quick      # subset (status, config, ping)
./benchmarks/run.sh --formats    # compare all output formats
./benchmarks/run.sh --json       # raw JSON output
```

### CI/CD

- **CI** (`.github/workflows/ci.yml`): check, test, clippy, fmt — runs on push/PR to main
- **Release** (`.github/workflows/release.yml`): cross-platform builds (Linux x86_64, macOS x86_64/arm64, Windows x86_64) + GitHub Release — triggers on `v*` tags

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `NUCLEO_TOKEN` | Bearer token (skips credentials file) |
| `NUCLEO_PROJECT_ID` | Override project_id from context |
| `NUCLEO_ENV_ID` | Override env_id from context |
| `NUCLEO_STAGE` | Override stage from context |
| `NUCLEO_API_KEY` | Override api_key from context |
| `NUCLEO_{KEY}_URL` | Override any service URL (e.g. `NUCLEO_AUTH_URL`) |
| `NUCLEO_TEMPLATES_DIR` | Override templates directory |

## Extension Guide

### Adding a New Command

1. Create `src/commands/<name>.rs`:

```rust
use clap::Args;
use crate::{client, config, error::CliError, formatter::{self, OutputFormat}};

#[derive(Args, Debug)]
pub struct MyCommandArgs {
    #[arg(long, default_value = "json")]
    format: String,
}

pub async fn handle(args: &MyCommandArgs) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.get(&format!("{url}/endpoint")).bearer_auth(token)
    }).await?;

    let body = client::handle_api_response(resp).await?;
    println!("{}", formatter::format_value(&body, &OutputFormat::from_str(&args.format)));
    Ok(())
}
```

2. Add `pub mod <name>;` to `src/commands/mod.rs`
3. Add variant to `Command` enum in `src/main.rs` and dispatch it

### Adding a Plugin

1. Create `plugins/<name>/` with `plugin.json` manifest
2. Write entrypoint — receives subcommand + args as process args
3. Read `{PREFIX}_*` env vars for token, URLs, and context
4. Write JSON to stdout, use exit codes 0/1/2/3/5
5. Install: `nucleo plugins install ./plugins/<name>`

### Adding an MCP Tool

Add a params struct and `#[tool]` method to `src/mcp/tools.rs`:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct MyParams {
    #[schemars(description = "Parameter description")]
    my_arg: Option<String>,
}

#[tool(name = "nucleo_my_tool", description = "What the tool does")]
async fn tool_my_tool(&self, Parameters(params): Parameters<MyParams>) -> String {
    let mut args = vec!["my-command", "--format", "json"];
    if let Some(val) = &params.my_arg {
        args.push("--my-arg");
        args.push(val);
    }
    self.run(&args).await
}
```

### Adding a Template

1. Create `templates/my-template/` with `{{project_name}}` and `{{key_url}}` placeholders
2. Scaffold: `nucleo plugins scaffold create my-project my-template`

### Adding an Environment Preset

Edit `~/.config/nucleo/config.json` — add to the `presets` object:

**Basic auth (legacy flat format):**
```json
{
  "presets": {
    "staging": {
      "auth": "https://auth.staging.example.com/api/v2",
      "api": "https://api.staging.example.com/api/v1"
    }
  }
}
```

**OAuth2 (structured format):**
```json
{
  "presets": {
    "staging": {
      "urls": { "api": "https://api.staging.example.com/v1" },
      "auth_method": "oauth2",
      "oauth2": {
        "client_id": "your-client-id",
        "authorize_url": "https://auth.staging.example.com/authorize",
        "token_url": "https://auth.staging.example.com/token",
        "scopes": ["read", "write"]
      }
    }
  }
}
```

Switch: `nucleo config env use staging`

## How to Fork

1. Clone the repo
2. Edit `src/consts.rs` — change 4 constants (`APP_NAME`, `APP_DIR`, `APP_PREFIX`, `APP_BIN`)
3. Update `Cargo.toml` — change `name`, `description`, `[[bin]] name`
4. Copy `config.json` to `~/.config/<your-app>/config.json` with your service URLs
5. Replace example commands (`ping`, `echo`) with your domain commands
6. Add your MCP tools, plugins, and templates
7. `cargo build --release`

## Testing Conventions

- Unit tests in `#[cfg(test)]` modules at the bottom of each file
- No integration tests against live APIs
- `tempfile` crate available in `[dev-dependencies]` for filesystem tests
- Env-var-dependent tests set/remove vars inline
