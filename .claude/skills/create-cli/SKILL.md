---
name: create-cli
description: Scaffold a production-ready CLI from any API in minutes — OpenAPI discovery, auto-generated commands, auth pre-configured, tests and MCP tools included
---

# /create-cli

Turn any API into a production-ready, MCP-enabled CLI in minutes. Provide a name, a description, and an API source — Claude handles the rest: discovers endpoints from OpenAPI specs or built-in profiles, pre-configures auth, generates command files with tests, writes MCP tools, and verifies the build.

> **Philosophy:** confirm-not-configure. Claude pre-fills everything it can; you only answer what Claude cannot infer.

---

## Well-Known API Profiles

Built-in profiles for these APIs. Use the API name as your target input (e.g. "Spotify", "GitHub").

| API | Base URL | Auth | authorize_url | token_url | Starter Commands |
|-----|----------|------|---------------|-----------|-----------------|
| Spotify | `api.spotify.com/v1` | OAuth2 PKCE | `accounts.spotify.com/authorize` | `accounts.spotify.com/api/token` | player, playlists, search, tracks, recently-played |
| GitHub | `api.github.com` | OAuth2/Bearer | `github.com/login/oauth/authorize` | `github.com/login/oauth/access_token` | repos, issues, pulls, user, gists |
| Stripe | `api.stripe.com/v1` | Bearer (API key) | — | — | customers, charges, invoices, subscriptions |
| Slack | `slack.com/api` | OAuth2/Bearer | `slack.com/oauth/v2/authorize` | `slack.com/api/oauth.v2.access` | messages, channels, users, files |
| Discord | `discord.com/api/v10` | OAuth2/Bearer | `discord.com/oauth2/authorize` | `discord.com/api/oauth2/token` | guilds, channels, messages, users |
| OpenAI | `api.openai.com/v1` | Bearer (API key) | — | — | chat, completions, models, images |
| Anthropic | `api.anthropic.com/v1` | Bearer (API key) | — | — | messages, models |
| Notion | `api.notion.com/v1` | OAuth2/Bearer | `api.notion.com/v1/oauth/authorize` | `api.notion.com/v1/oauth/token` | pages, databases, blocks, users |
| Linear | `api.linear.app` | OAuth2/Bearer | `linear.app/oauth/authorize` | `api.linear.app/oauth/token` | issues, projects, teams, cycles |
| Vercel | `api.vercel.com` | Bearer (API key) | — | — | deployments, projects, domains, env |
| Google | `www.googleapis.com` | OAuth2 | `accounts.google.com/o/oauth2/v2/auth` | `oauth2.googleapis.com/token` | (service-specific) |
| Cloudflare | `api.cloudflare.com/client/v4` | Bearer (API key) | — | — | zones, dns, workers, pages |
| Twilio | `api.twilio.com/2010-04-01` | Basic (SID+secret) | — | — | messages, calls, accounts |
| Twitter/X | `api.twitter.com/2` | OAuth2 | `twitter.com/i/oauth2/authorize` | `api.twitter.com/2/oauth2/token` | tweets, users, search, timelines |
| PlanetScale | `api.planetscale.com/v1` | Bearer (API key) | — | — | databases, branches, deploy-requests |

---

## Phase 0: Bootstrap

Before anything else, clone the nucleo core so the user has a fresh foundation to build on.

### 0.1 — Determine destination

Ask exactly **1 question**:

```
Where should the new CLI be created?
  Path (default: ./{cli-name}): _
```

If the user presses Enter, use `./{cli-name}` — resolved after Phase 1 collects the name.

### 0.2 — Clone nucleo core

```bash
git clone https://github.com/mateonunez/nucleo.git {destination}
```

After cloning, remove the `.git` directory so the new project starts with a clean history:

```bash
cd {destination} && rm -rf .git && git init && git add -A && git commit -m "chore: init from nucleo"
```

Also remove the existing `config.json` at the repo root (it will be regenerated in Phase 5):

```bash
rm -f config.json
```

Print confirmation:
```
✓ nucleo cloned into {destination}
✓ Git history reset — fresh repo initialized
Working directory: {destination}
```

All subsequent file edits happen inside `{destination}`.

---

## Phase 1: Identity

Ask exactly **3 questions** — nothing more (destination was already asked in Phase 0):

```
1. CLI name         e.g. "spotify-cli"
2. Description      e.g. "Control Spotify from the terminal"
3. Target API       one of:
                    - Well-known name:  "Spotify"
                    - OpenAPI URL:      https://api.example.com/openapi.json
                    - Local spec path:  ./openapi.yaml
                    - Raw base URL:     https://api.example.com/v1
```

Once the CLI name is known, resolve the destination from Phase 0 (e.g. default `./{cli-name}`) and proceed with the clone if not yet done.

**Auto-derive from CLI name** (no prompts):
- `APP_NAME` → cli name as-is (e.g. `spotify-cli`)
- `APP_DIR`  → cli name as-is (e.g. `spotify-cli`)
- `APP_BIN`  → cli name as-is (e.g. `spotify-cli`)
- `APP_PREFIX` → uppercase, hyphens → underscores (e.g. `SPOTIFY_CLI`)

**Auto-detect author** from `git config user.name` — run `git config user.name` silently. If empty, use `"unknown"`.

---

## Phase 2: API Discovery

Act as an intelligent API researcher. Try multiple strategies, chain them, and adapt to whatever the user provides. The goal: discover endpoints, auth methods, and API structure with minimal user effort.

### Strategy A — Well-Known Name

Match the input (case-insensitive) against the profile table above.

Load the matching profile as a **starting point**:
- `base_url`, `auth_type`, `authorize_url`, `token_url`, `starter_commands`

Then **also attempt live discovery** (Strategy B or C) to enrich beyond the starter set. The profile accelerates — it doesn't limit.

Present a summary and ask the user to confirm or adjust:

```
Found profile: Spotify
  Base URL:  https://api.spotify.com/v1
  Auth:      OAuth2 PKCE
  Authorize: https://accounts.spotify.com/authorize
  Token:     https://accounts.spotify.com/api/token
  Starter commands: player, playlists, search, tracks, recently-played

  Enrichment: also discovered 47 endpoints via live API docs.

Looks good? [Y/n]
```

### Strategy B — OpenAPI / Swagger Spec

If input is a URL: use `WebFetch` to read it.
If input is a local file path: use `Read` to load it.

From the spec, extract:
1. `info.title`, `info.version` (for display)
2. `servers[0].url` → `base_url`
3. `components.securitySchemes` / `securityDefinitions` → `auth_type` detection
4. All `paths` entries → group by first tag, or by first path segment if untagged

For each path entry, extract:
- HTTP method + path
- `operationId` (use as command name candidate)
- `summary` / `description`
- `parameters` → path params (required args), query params (optional flags)
- `requestBody` → `--data <json>` flag + any named `--field` shortcuts for top-level fields
- Whether the operation has a security requirement

Supports OpenAPI 2.0 (Swagger), 3.0, and 3.1 formats (JSON or YAML).

Group by resource tag. Present summary:

```
Discovered 87 endpoints across 14 resources.

Resources:
  albums      (6 endpoints)
  artists     (7 endpoints)
  player      (13 endpoints)
  playlists   (8 endpoints)
  search      (1 endpoint)
  tracks      (8 endpoints)
  users       (3 endpoints)
  ... (7 more)

Continue to command selection? [Y/n]
```

### Strategy C — Live API Documentation URL

When the user provides any documentation URL (not a raw spec), fetch and parse it intelligently.

1. **Fetch** the page with `WebFetch`
2. **Identify** the docs platform and extract API info accordingly:
   - **Swagger UI / Redoc / Stoplight:** Look for embedded OpenAPI spec in page source (`<script>` tags, `/swagger.json` or `/openapi.json` links). If found, switch to Strategy B.
   - **ReadMe.io / Mintlify / GitBook:** Parse the structured content for endpoint listings, method badges, path patterns, parameter tables, and auth sections
   - **Postman published collections:** Extract endpoints, methods, headers, example request/response bodies
   - **Custom HTML docs:** Scan for endpoint tables, curl examples, code blocks, and REST patterns
3. **Follow navigation** — if docs span multiple pages (sidebar links, pagination), follow links to discover all endpoint categories
4. **Extract from code examples** — curl commands are especially rich:
   - `curl -X POST https://api.example.com/v1/users` → POST /users
   - `-H "Authorization: Bearer ..."` → bearer auth
   - `-d '{"name": "..."}'` → request body shape
5. **Note rate limits** and pagination patterns mentioned in docs text

Present the same structured summary as Strategy B.

### Strategy D — Base URL Auto-Discovery

Given just a base URL (e.g., `https://api.example.com/v1`):

Probe common discovery endpoints in order (use `WebFetch` for each):
1. `{base}/openapi.json`, `{base}/swagger.json`
2. `{base}/../openapi.json`, `{base}/../swagger.json`
3. `{base}/.well-known/openapi`
4. `{base}/docs`, `{base}/api-docs`, `{base}/redoc`
5. `{base}/swagger-ui`, `{base}/swagger-ui/index.html`
6. `{base}/` (some APIs return a hypermedia resource listing)

If a spec is found → switch to Strategy B.
If a docs page is found → switch to Strategy C.
If nothing found → switch to Strategy E.

### Strategy E — Manual with AI Assistance

When no docs are discoverable, help the user interactively:

1. **Ask context:** "What does this API do? What are the main resources/entities?"
2. **Suggest endpoints** from the description:
   - "Based on your description, this API likely has: users, projects, tasks. Shall I generate standard CRUD commands (list, get, create, update, delete) for each?"
3. **Accept examples:** "Do you have any curl examples, Postman exports, or sample API responses I can analyze?"
   - Reverse-engineer endpoints from curl commands, exported collections, or response shapes
4. **Ask about auth:** "What authentication does this API use? (OAuth2, API key, basic auth, none)"

Build the endpoint list collaboratively, confirming with the user at each step.

### Combining Strategies

Strategies chain naturally — use the best tool for each piece of information:

- **A + C:** Well-known profile provides auth config → live docs fetch discovers full endpoint list
- **D + C → B:** Base URL probe finds a Swagger UI page → parse it → find embedded spec → process spec
- **A + E:** Well-known profile as starting point → user adds custom/internal endpoints manually
- **E + B:** User provides Postman export → Claude parses it as a pseudo-spec

### Discovery Summary

After discovery (regardless of strategy), always present:

```
API Discovery Results
  Base URL:      https://api.spotify.com/v1
  Auth method:   OAuth2 PKCE [high confidence]
  Endpoints:     47 across 12 resources
  Top resources: player (8), playlists (7), tracks (5), albums (4), search (1)
  Pagination:    offset/limit style detected
  Source:        well-known profile + live docs enrichment

Proceed to auth configuration? [Y/n]
```

If confidence is low on any aspect, flag it explicitly and ask the user to confirm.

---

## Phase 3: Auth Configuration

Based on the detected auth type, **pre-fill everything you can** and collect credentials interactively.

### OAuth2 (PKCE)

Pre-fill from profile or spec:
- `authorize_url`
- `token_url`
- `redirect_path` → `/callback` (default)
- `scopes` → from profile or spec security scheme

Show registration instructions **before** asking for credentials:

```
OAuth2 setup for {Provider}

  1. Go to {provider-dashboard-url}
  2. Create a new application
  3. Set redirect URI to: http://127.0.0.1:8888/callback
     (nucleo uses port 8888 for the local callback server — register this exact URI)
  4. Copy your client_id and client_secret below
```

Then ask — **both fields are required to proceed**:

```
Client ID (required):
> _

Client Secret (optional — leave blank for public PKCE clients):
> _
```

**IMPORTANT:** Do NOT leave `client_id` empty in the generated config. If the user skips it, stop and explain:
```
⚠ client_id is required for OAuth2 login to work.
  Add it now, or fill it in manually before running `{cli-name} auth login`.
```

Generate `config.json` with the collected values:
```json
{
  "urls": {},
  "active_env": "production",
  "presets": {
    "production": {
      "urls": {
        "api": "{base_url}"
      },
      "auth_method": "oauth2",
      "oauth2": {
        "client_id": "{client_id}",
        "client_secret": "{client_secret_or_omit_field}",
        "authorize_url": "{authorize_url}",
        "token_url": "{token_url}",
        "redirect_path": "/callback",
        "scopes": ["{scope1}", "{scope2}"]
      }
    }
  },
  "plugins": { "directory": null, "registries": [] }
}
```

If `client_secret` was left blank, **omit the field entirely** from the generated JSON (do not write `"client_secret": ""`).

### Bearer / API Key

No prompts needed. Configure via env var at runtime.

Generate `config.json`:
```json
{
  "urls": {},
  "active_env": "production",
  "presets": {
    "production": {
      "api": "{base_url}"
    }
  },
  "plugins": { "directory": null, "registries": [] }
}
```

Generate `.env.example`:
```
# Set your API token (overrides stored credentials)
{PREFIX}_TOKEN=your_api_key_here

# Override service URLs
{PREFIX}_API_URL=
```

**Important for API-key auth:** Use `send_with_retry` with manual `.bearer_auth(token)` instead of `send_authenticated`. Read the token from env var `{PREFIX}_TOKEN` or from stored credentials.

### Basic Auth (API Key as SID+secret, e.g. Twilio)

Generate `config.json` with `auth` URL:
```json
{
  "urls": {},
  "active_env": "production",
  "presets": {
    "production": {
      "auth": "{auth_url}",
      "api": "{base_url}"
    }
  },
  "plugins": { "directory": null, "registries": [] }
}
```

### Multiple Schemes

Present options and let user pick one. Then follow the chosen scheme above.

---

## Phase 4: Command Selection

Present discovered endpoints grouped by resource. Pre-select sensible defaults (GET-heavy starter set, top 15–20 for large APIs).

```
Select commands to generate:

player (13 endpoints):
  [x] get-current   GET  /me/player                  -- Get current playback
  [x] play          PUT  /me/player/play              -- Start/resume playback
  [x] pause         PUT  /me/player/pause             -- Pause playback
  [x] next          POST /me/player/next              -- Skip to next track
  [ ] queue-add     POST /me/player/queue             -- Add item to queue
  [ ] devices       GET  /me/player/devices           -- Get available devices

playlists (8 endpoints):
  [x] list          GET  /me/playlists                -- Get user's playlists
  [x] get           GET  /playlists/{id}              -- Get playlist details
  [ ] create        POST /users/{id}/playlists        -- Create playlist
  [ ] add-tracks    POST /playlists/{id}/tracks       -- Add tracks to playlist

...

[Enter to confirm, or type resource names to toggle all, or +/- individual items]
```

For each **selected** command, determine:

| Source | Maps to |
|--------|---------|
| Path param `{id}` | Required positional `<id>` in `Args` |
| Required query param | `#[arg(long)]` required flag |
| Optional query param | `#[arg(long)]` optional flag with `Option<T>` |
| Paginated list response | Use Template C; add `--limit`, `--offset`, `--all` |
| Request body present | `#[arg(long)] data: Option<String>` for raw JSON + named `--field` for top-level string fields |
| Operation requires auth | Use Template B, C, or D |
| Operation is public | Use Template A |

Decide which template to use per command:
- Single resource + multiple operations → **Template E** (subcommand)
- List endpoint with pagination → **Template C**
- Mutation (POST/PUT/PATCH/DELETE) → **Template D**
- Simple authenticated GET → **Template B**
- Public unauthenticated GET → **Template A**

---

## Phase 5: Generate Everything

Run `cargo check` after steps 5.1, 5.2, 5.4, and 5.7.

### 5.1 — Brand the fork

Edit `src/consts.rs`:
```rust
pub const APP_NAME: &str = "{cli-name}";
pub const APP_DIR: &str = "{cli-name}";
pub const APP_PREFIX: &str = "{CLI_PREFIX}";
pub const APP_BIN: &str = "{cli-name}";
```

Edit `Cargo.toml`:
```toml
[package]
name = "{cli-name}"
description = "{description}"
authors = ["{author}"]

[[bin]]
name = "{cli-name}"
path = "src/main.rs"
```

Edit `src/main.rs` — update the `#[command(...)]` attribute on the `Cli` struct:
```rust
#[command(
    name = "{cli-name}",
    version,
    about = "{cli-name} — {description}",
    long_about = None,
    arg_required_else_help = true
)]
```

Also update the `generate()` call in the `Completions` handler — change the hardcoded `"nucleo"` to the new binary name:
```rust
generate(*shell, &mut Cli::command(), "{cli-name}", &mut io::stdout());
```

Run `cargo check`.

### 5.2 — Remove example commands

1. `rm src/commands/ping.rs src/commands/echo.rs`
2. Remove `pub mod ping;` and `pub mod echo;` from `src/commands/mod.rs`
3. Remove `Ping(ping::PingArgs)` and `Echo(echo::EchoArgs)` variants from the `Command` enum in `src/main.rs`
4. Remove their `Command::Ping(args) => ping::handle(args).await` and `Command::Echo(args) => echo::handle(args).await` match arms

Run `cargo check`.

### 5.3 — Generate command files

For each selected command, write `src/commands/{name}.rs` using the appropriate template below.

Apply these substitutions in every template:
- `{Name}` → PascalCase struct name (e.g. `PlaylistsArgs`, `PlaylistsCommand`)
- `{name}` → snake_case command name (e.g. `playlists`)
- `{resource}` → lowercase plural resource name for URL paths (e.g. `playlists`, `users`)
- `{endpoint}` → the API path with Rust format string syntax (e.g. `"/playlists/{id}"` → `format!("{url}/playlists/{id}", id=args.id)`)
- `{method}` → HTTP method call **including the leading dot** (e.g. `.get`, `.post`, `.put`, `.delete`, `.patch`) — so `http{method}(...)` becomes `http.get(...)`
- `{PREFIX}` → the CLI prefix constant (e.g. `SPOTIFY_CLI`)

### 5.4 — Register commands

For each generated command file:

1. Add to `src/commands/mod.rs`:
   ```rust
   pub mod {name};
   ```

2. Add variant to `Command` enum in `src/main.rs`:
   ```rust
   /// {command description}
   {PascalName}({name}::{Name}Args),
   // or for subcommand pattern:
   {PascalName} { #[command(subcommand)] cmd: {name}::{Name}Command },
   ```

3. Add dispatch match arm in `src/main.rs`:
   ```rust
   Command::{PascalName}(args) => {name}::handle(args).await?,
   // or for subcommand pattern:
   Command::{PascalName} { cmd } => {name}::handle(&cmd).await?,
   ```

Run `cargo check`.

### 5.5 — Generate config.json

Write `config.json` at the project root using the structure from Phase 3.

After writing, determine the user's config directory and print the correct install command:

```bash
# Detect OS for the correct config path
uname_out=$(uname -s 2>/dev/null || echo "Linux")
if [ "$uname_out" = "Darwin" ]; then
  config_dir="$HOME/.config/{cli-name}"
else
  config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/{cli-name}"
fi
echo "Config directory: $config_dir"
```

Then print:
```
Config written to config.json.

To install:
  mkdir -p {config_dir}
  cp config.json {config_dir}/config.json
```

**Config directory facts (important — the CLI always uses `~/.config`, NOT `~/Library/Application Support`):**
- macOS: `~/.config/{cli-name}/config.json`
- Linux: `~/.config/{cli-name}/config.json` (or `$XDG_CONFIG_HOME/{cli-name}/config.json`)
- Override anytime: set `{PREFIX}_CONFIG_DIR=/path/to/dir`

### 5.6 — Generate .env.example

Write `.env.example` at the project root:
```
# {cli-name} environment overrides
# These override values in config.json

# API token (skips login prompt)
{PREFIX}_TOKEN=

# Override service URLs
{PREFIX}_API_URL=

# Override config directory (default: ~/.config/{cli-name})
{PREFIX}_CONFIG_DIR=

# Active environment preset
{PREFIX}_ENV=production
```

### 5.7 — Generate MCP tools

Overwrite `src/mcp/tools.rs`. Keep the existing `NucleoServer` struct, `run`, and `run_owned` helpers. Replace the example tools with one tool per generated command:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct {Name}Params {
    // Mirror the command's Args fields as Option<T>
    // e.g. for a list command with --limit:
    pub limit: Option<u32>,
}

#[tool(
    name = "{cli_name}_{command_name}",
    description = "{command description}"
)]
async fn tool_{command_name}(&self, Parameters(params): Parameters<{Name}Params>) -> String {
    let mut args = vec![
        "{command-name}".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    // Append optional params
    if let Some(limit) = params.limit {
        args.push("--limit".to_string());
        args.push(limit.to_string());
    }
    self.run_owned(&args).await
}
```

Update `src/mcp/mod.rs` — find the `ServerInfo` / `Implementation` block and change the server name, version, and instructions to match the new CLI. Read the actual file first to match the exact pattern used by rmcp.

### 5.8 — Generate tests

Append a `#[cfg(test)]` block to each generated command file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse_defaults() {
        use clap::Parser;
        // Wrap Args in a minimal CLI struct for parsing
        #[derive(clap::Parser)]
        struct Cli { #[command(flatten)] args: {Name}Args }
        let cli = Cli::parse_from(["cmd"]);
        assert_eq!(cli.args.format, "json");
    }

    #[test]
    fn args_parse_with_flags() {
        use clap::Parser;
        #[derive(clap::Parser)]
        struct Cli { #[command(flatten)] args: {Name}Args }
        let cli = Cli::parse_from(["cmd", "--format", "table"]);
        assert_eq!(cli.args.format, "table");
    }
}
```

For Template E (subcommand), test each subcommand variant parses correctly.

### 5.9 — Update README.md

Replace the existing README with:
1. `# {cli-name}` heading + description
2. Installation section (`cargo install --path .`)
3. Setup section (`{cli-name} setup`)
4. Auth section (OAuth2 registration steps if applicable, or API key setup)
5. Command tree (generated from the actual commands created)
6. Configuration section (config.json structure, env vars) — include the correct config path (`~/.config/{cli-name}/`)
7. MCP integration section

---

## Phase 6: Verify & Guide

### Verification

Run in order — stop and fix if any step fails:

```bash
cargo check
cargo test
cargo clippy -- -D warnings
```

If `cargo check` fails after 5.1: re-read `src/main.rs` and fix the `Cli` struct.
If `cargo check` fails after 5.2: check that all `ping`/`echo` references are removed.
If `cargo check` fails after 5.4: check the `Command` enum variants match the generated `Args` types exactly.
If `cargo test` fails: fix the test helper structs (the `#[derive(clap::Parser)]` wrapper pattern).
If `cargo clippy` has warnings: fix them — do not use `#[allow(dead_code)]` as a workaround.

Print the command tree:
```bash
cargo run -- --help
```

### Next Steps Checklist

Print this numbered checklist after a successful build:

```
✓ {cli-name} is ready.

Next steps:

  1. {if OAuth2} Register an OAuth2 app at {provider-dashboard-url}
               Set redirect URI: http://127.0.0.1:8888/callback
               ↑ This exact URI — nucleo uses port 8888 for the local callback server.
               Copy your client_id (and client_secret if required) into:
               ~/.config/{cli-name}/config.json → presets.production.oauth2.client_id

  2. Copy config to your config directory:
       mkdir -p ~/.config/{cli-name}
       cp config.json ~/.config/{cli-name}/config.json

  3. Install the binary:
       cargo install --path .

  4. Run setup:
       {cli-name} setup

  5. Authenticate:
       {cli-name} auth login

  6. Run your first command:
       {cli-name} {first-generated-command} --format table

Troubleshooting:
  "No presets defined"       → config.json is not in ~/.config/{cli-name}/ (see step 2)
  "client_id is empty"       → add your OAuth2 client_id to config.json (see step 1)
  "client_secret is empty"   → either remove the field or add the real secret from your dashboard
  "Invalid redirect URI"     → register http://127.0.0.1:8888/callback in your provider dashboard
  Config dir location        → run `{cli-name} config show` to see the exact path being used
  Override config dir        → set {PREFIX}_CONFIG_DIR=/path/to/dir
```

---

## Code Templates

Use these verbatim — substitute `{Name}`, `{name}`, `{endpoint}`, `{method}`, `{PREFIX}` as described in 5.3.

### Template A — Unauthenticated GET

```rust
use clap::Args;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct {Name}Args {
    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &{Name}Args) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}{endpoint}");

    let http = client::build_client()?;
    let resp = client::send_with_retry(|| http{method}(&endpoint))
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Request failed: {e}")))?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}
```

### Template B — Authenticated GET

```rust
use clap::Args;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct {Name}Args {
    // Path params (one per {param} in the endpoint):
    // pub id: String,

    // Optional query params:
    // #[arg(long)] pub filter: Option<String>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &{Name}Args) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}{endpoint}"); // substitute path params from args

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http{method}(&endpoint).bearer_auth(token)
    })
    .await?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}
```

### Template C — Authenticated GET with Pagination

```rust
use clap::Args;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct {Name}Args {
    /// Maximum number of items to return per page
    #[arg(long, default_value = "20")]
    pub limit: u32,

    /// Offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: u32,

    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &{Name}Args) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let base_endpoint = format!("{url}{endpoint}");

    let http = client::build_client()?;
    let fmt = OutputFormat::from_str(&args.format);

    if args.all {
        let mut all_items = serde_json::Value::Array(vec![]);
        let mut offset = 0u32;
        loop {
            let endpoint = format!("{base_endpoint}?limit={}&offset={}", args.limit, offset);
            let resp = client::send_authenticated(&http, |token| {
                http.get(&endpoint).bearer_auth(token)
            })
            .await?;
            let body = client::handle_api_response(resp).await?;

            // Extract items array — adjust key to match actual API response shape
            let items = body.get("items")
                .or_else(|| body.get("data"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let count = items.len();
            if let serde_json::Value::Array(ref mut arr) = all_items {
                arr.extend(items);
            }
            if count < args.limit as usize {
                break;
            }
            offset += args.limit;
        }
        println!("{}", formatter::format_value(&all_items, &fmt));
    } else {
        let endpoint = format!("{base_endpoint}?limit={}&offset={}", args.limit, args.offset);
        let resp = client::send_authenticated(&http, |token| {
            http.get(&endpoint).bearer_auth(token)
        })
        .await?;
        let body = client::handle_api_response(resp).await?;
        println!("{}", formatter::format_value(&body, &fmt));
    }

    Ok(())
}
```

### Template D — Authenticated Mutation (POST / PUT / PATCH / DELETE)

```rust
use clap::Args;
use serde_json::Value;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct {Name}Args {
    // Path params:
    // pub id: String,

    /// Raw JSON body (overrides individual --field flags)
    #[arg(long)]
    pub data: Option<String>,

    // Named field shortcuts (add one per common top-level field):
    // #[arg(long)] pub name: Option<String>,
    // #[arg(long)] pub description: Option<String>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &{Name}Args) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}{endpoint}");

    let body: Value = if let Some(ref raw) = args.data {
        serde_json::from_str(raw)
            .map_err(|e| CliError::Validation(format!("Invalid JSON in --data: {e}")))?
    } else {
        let mut obj = serde_json::Map::new();
        // Populate from named fields:
        // if let Some(ref name) = args.name { obj.insert("name".into(), Value::String(name.clone())); }
        Value::Object(obj)
    };

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http{method}(&endpoint).bearer_auth(token).json(&body)
    })
    .await?;

    let result = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&result, &fmt));
    Ok(())
}
```

### Template E — Resource with Subcommands

```rust
use clap::Subcommand;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Subcommand, Debug)]
pub enum {Name}Command {
    /// List all {resource}
    List {
        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Get a specific {resource} by ID
    Get {
        /// {Resource} ID
        id: String,
        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },
    // Add more subcommands as needed
}

pub async fn handle(cmd: &{Name}Command) -> Result<(), CliError> {
    match cmd {
        {Name}Command::List { format } => list(format).await,
        {Name}Command::Get { id, format } => get(id, format).await,
    }
}

async fn list(format: &str) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/{resource}");

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.get(&endpoint).bearer_auth(token)
    })
    .await?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}

async fn get(id: &str, format: &str) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/{resource}/{id}");

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.get(&endpoint).bearer_auth(token)
    })
    .await?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}
```

---

## Rules & Constraints

These rules apply to ALL generated code and config. Never violate them.

**Framework preservation:**
- Keep ALL framework infrastructure: `error.rs`, `formatter.rs`, `config.rs`, `client.rs`, plugin system, MCP server
- Keep ALL framework commands: `auth`, `config`, `status`, `completions`, `plugins`, `mcp`, `setup`
- Only replace domain-specific parts: `consts.rs` values, `ping.rs`, `echo.rs`, new command files, `config.json`, `mcp/tools.rs`

**Code quality:**
- Never use `panic!`, `unwrap()`, or `expect()` in generated command code
- All `handle` functions return `Result<(), CliError>`
- Use `serde_json::Value` for all API responses — no typed response structs
- All `CliError` variants must match the ones defined in `src/error.rs`

**Auth patterns:**
- OAuth2 APIs: use `client::send_authenticated(&http, |token| req.bearer_auth(token))`
- API key / Bearer token APIs: use `client::send_with_retry(|| req.bearer_auth(&token))` where `token` is read from env `{PREFIX}_TOKEN` or from `config::load_credentials()`
- Never hardcode tokens or secrets
- **Never generate a config with `"client_id": ""`** — always either fill it with the real value or use a visible placeholder like `"YOUR_CLIENT_ID_HERE"` with a comment in the next steps

**Config directory:**
- The CLI always resolves config to `~/.config/{cli-name}/` (NOT `~/Library/Application Support` on macOS)
- Always use this path in docs, README, and next steps output
- Users can override with `{PREFIX}_CONFIG_DIR` env var or `XDG_CONFIG_HOME`

**OAuth2 redirect URI:**
- The callback server binds to port **8888** by default (fixed, not random)
- Always tell users to register exactly: `http://127.0.0.1:8888/callback`
- If port 8888 is busy, the server falls back to a random port — warn the user if this happens

**Build hygiene:**
- Run `cargo check` after 5.1, 5.2, 5.4, and 5.7
- Run `cargo test` and `cargo clippy -- -D warnings` in Phase 6
- Fix all errors and warnings before proceeding to the next phase
- Do not use `#[allow(dead_code)]` or `#[allow(unused)]` as workarounds
