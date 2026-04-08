# Create a New CLI with nucleo

Build a production-ready CLI for any API in minutes. This guide walks you through turning nucleo into your own branded CLI tool — whether you're building a Spotify controller, a GitHub manager, or a wrapper for your company's internal API.

## What You'll Get

By the end of this guide, your CLI will have:

- Commands that talk to your API (with auth, pagination, and error handling)
- OAuth2 login (or API key auth — your choice)
- Multiple output formats (JSON, table, YAML, CSV)
- Shell completions (bash, zsh, fish, powershell)
- Claude Desktop integration via MCP
- A plugin system for community extensions
- CI/CD with GitHub Actions

## Prerequisites

- **Rust 1.85+** — [install via rustup](https://rustup.rs/)
- **Git** — to clone the repo
- **An API to wrap** — any REST API with documentation

## Quick Path vs. Full Guide

| I want to... | Do this |
|--------------|---------|
| Build a CLI for a well-known API (Spotify, GitHub, Stripe, etc.) | Jump to [Option A: Well-Known API](#option-a-well-known-api) |
| Build a CLI from an OpenAPI/Swagger spec | Jump to [Option B: From an API Spec](#option-b-from-an-api-spec) |
| Build a CLI for a custom/internal API | Jump to [Option C: Custom API](#option-c-custom-api) |
| Use Claude Code to do it all automatically | Jump to [The Fast Way: Claude Code](#the-fast-way-claude-code) |

---

## Step 1: Clone and Set Up

```sh
git clone https://github.com/mateonunez/nucleo.git my-cli
cd my-cli
cargo build  # verify everything compiles
```

## Step 2: Choose Your Identity

Every nucleo CLI is defined by 4 constants. Pick your names:

| Constant | What it does | Example (Spotify) |
|----------|-------------|-------------------|
| `APP_NAME` | Display name in help text | `spotify-cli` |
| `APP_DIR` | Config folder name (`~/.config/<this>/`) | `spotify-cli` |
| `APP_PREFIX` | Env var prefix (`<THIS>_TOKEN`, etc.) | `SPOTIFY_CLI` |
| `APP_BIN` | Binary name | `spotify-cli` |

Edit **`src/consts.rs`**:

```rust
pub const APP_NAME: &str = "spotify-cli";
pub const APP_DIR: &str = "spotify-cli";
pub const APP_PREFIX: &str = "SPOTIFY_CLI";
pub const APP_BIN: &str = "spotify-cli";
```

Edit **`Cargo.toml`**:

```toml
[package]
name = "spotify-cli"
description = "Control Spotify from the terminal"

[[bin]]
name = "spotify-cli"
path = "src/main.rs"
```

Edit **`src/main.rs`** — update the CLI struct:

```rust
#[command(
    name = "spotify-cli",
    version,
    about = "spotify-cli — Control Spotify from the terminal",
    long_about = None,
    arg_required_else_help = true
)]
```

Also update the shell completions line in the same file:

```rust
generate(*shell, &mut Cli::command(), "spotify-cli", &mut io::stdout());
```

Verify it compiles:

```sh
cargo check
```

## Step 3: Remove Example Commands

nucleo ships with two example commands (`ping` and `echo`) that demonstrate the HTTP patterns. Remove them:

1. Delete the files:
   ```sh
   rm src/commands/ping.rs src/commands/echo.rs
   ```

2. Edit **`src/commands/mod.rs`** — remove these lines:
   ```rust
   pub mod echo;
   pub mod ping;
   ```

3. Edit **`src/main.rs`** — remove the `Ping` and `Echo` variants from the `Command` enum and their match arms in the dispatch block.

4. Verify:
   ```sh
   cargo check
   ```

## Step 4: Configure Authentication

nucleo supports three auth methods out of the box. Choose the one your API uses.

### Option A: Well-Known API

If you're building for one of these APIs, the auth details are ready to go:

| API | Auth Method | Registration URL |
|-----|-------------|------------------|
| Spotify | OAuth2 PKCE | [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) |
| GitHub | OAuth2 / Bearer | [github.com/settings/developers](https://github.com/settings/developers) |
| Stripe | API Key (Bearer) | [dashboard.stripe.com/apikeys](https://dashboard.stripe.com/apikeys) |
| Slack | OAuth2 | [api.slack.com/apps](https://api.slack.com/apps) |
| Discord | OAuth2 | [discord.com/developers/applications](https://discord.com/developers/applications) |
| OpenAI | API Key (Bearer) | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) |
| Anthropic | API Key (x-api-key) | [console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys) |
| Notion | OAuth2 | [notion.so/my-integrations](https://www.notion.so/my-integrations) |
| Linear | OAuth2 | [linear.app/settings/api](https://linear.app/settings/api) |
| Vercel | API Key (Bearer) | [vercel.com/account/tokens](https://vercel.com/account/tokens) |
| Google | OAuth2 | [console.cloud.google.com/apis/credentials](https://console.cloud.google.com/apis/credentials) |
| Cloudflare | API Key (Bearer) | [dash.cloudflare.com/profile/api-tokens](https://dash.cloudflare.com/profile/api-tokens) |
| Twilio | Basic Auth | [console.twilio.com](https://console.twilio.com) |
| Twitter/X | OAuth2 PKCE | [developer.twitter.com/en/portal](https://developer.twitter.com/en/portal) |

### Option B: From an API Spec

If your API has an OpenAPI or Swagger spec, check the `securitySchemes` section — it will tell you which auth method to use.

### Option C: Custom API

Ask your API provider which auth method they use, then follow the matching section below.

---

### Configuring OAuth2 (PKCE)

*For APIs like Spotify, GitHub, Slack, Discord, Google, Twitter/X, etc.*

This is the most common auth method for APIs that act on behalf of a user.

1. **Register your app** with the API provider (see the registration URL in the table above)
2. Set the **redirect URI** to `http://127.0.0.1:8888/callback`
3. Copy your **client_id**

4. Create your **`config.json`**:

```json
{
  "urls": {},
  "active_env": "production",
  "presets": {
    "production": {
      "urls": {
        "api": "https://api.spotify.com/v1"
      },
      "auth_method": "oauth2",
      "oauth2": {
        "client_id": "YOUR_CLIENT_ID_HERE",
        "authorize_url": "https://accounts.spotify.com/authorize",
        "token_url": "https://accounts.spotify.com/api/token",
        "redirect_path": "/callback",
        "scopes": [
          "user-read-playback-state",
          "playlist-read-private",
          "user-library-read"
        ]
      }
    }
  },
  "plugins": { "directory": null, "registries": [] }
}
```

5. Copy it to your config directory:
```sh
mkdir -p ~/.config/spotify-cli
cp config.json ~/.config/spotify-cli/config.json
```

When you run your CLI, users authenticate with:
```sh
spotify-cli auth login
# Opens browser → user authorizes → token saved automatically
```

### Configuring API Key / Bearer Token

*For APIs like Stripe, OpenAI, Vercel, Cloudflare, etc.*

No OAuth2 dance needed. Users just set an environment variable.

1. Create your **`config.json`**:

```json
{
  "urls": {
    "api": "https://api.openai.com/v1"
  },
  "active_env": "",
  "presets": {},
  "plugins": { "directory": null, "registries": [] }
}
```

2. Users authenticate by setting:
```sh
export OPENAI_CLI_TOKEN="sk-your-api-key-here"
```

Or by running:
```sh
openai-cli auth login
```

### Configuring Basic Auth

*For APIs like Twilio that use username:password.*

1. Create your **`config.json`** with an `auth` URL in the preset:

```json
{
  "urls": {},
  "active_env": "production",
  "presets": {
    "production": {
      "auth": "https://api.twilio.com/2010-04-01",
      "api": "https://api.twilio.com/2010-04-01"
    }
  },
  "plugins": { "directory": null, "registries": [] }
}
```

2. Users authenticate with:
```sh
twilio-cli auth login --username ACXXXXXXX
# Prompted for password securely
```

## Step 5: Write Your Commands

This is where your CLI comes to life. Each command is a Rust file in `src/commands/`.

### Simple Command (Authenticated GET)

This is the most common pattern — fetch data from your API and display it.

Create **`src/commands/playlists.rs`**:

```rust
use clap::Args;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct PlaylistsArgs {
    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &PlaylistsArgs) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/me/playlists");

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.get(&endpoint).bearer_auth(token)
    })
    .await?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}
```

### Command with Parameters

For endpoints that take IDs or query parameters:

Create **`src/commands/playlist.rs`**:

```rust
use clap::Args;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct PlaylistArgs {
    /// Playlist ID
    pub id: String,

    /// Maximum number of tracks to return
    #[arg(long, default_value = "20")]
    pub limit: u32,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &PlaylistArgs) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/playlists/{}/tracks", args.id);

    let http = client::build_client()?;
    let limit = args.limit;
    let resp = client::send_authenticated(&http, |token| {
        http.get(&endpoint)
            .bearer_auth(token)
            .query(&[("limit", limit.to_string())])
    })
    .await?;

    let body = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}
```

### Command with Subcommands

For resources that have multiple operations (list, get, create, etc.), use the subcommand pattern:

Create **`src/commands/player.rs`**:

```rust
use clap::Subcommand;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Subcommand, Debug)]
pub enum PlayerCommand {
    /// Get current playback state
    Status {
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Start or resume playback
    Play,
    /// Pause playback
    Pause,
}

pub async fn handle(cmd: &PlayerCommand) -> Result<(), CliError> {
    match cmd {
        PlayerCommand::Status { format } => status(format).await,
        PlayerCommand::Play => play().await,
        PlayerCommand::Pause => pause().await,
    }
}

async fn status(format: &str) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/me/player");

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

async fn play() -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/me/player/play");

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.put(&endpoint).bearer_auth(token)
    })
    .await?;

    client::handle_api_response(resp).await?;
    println!("Playback started.");
    Ok(())
}

async fn pause() -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/me/player/pause");

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.put(&endpoint).bearer_auth(token)
    })
    .await?;

    client::handle_api_response(resp).await?;
    println!("Playback paused.");
    Ok(())
}
```

### Command that Sends Data (POST/PUT)

For endpoints that accept a JSON body:

Create **`src/commands/create_playlist.rs`**:

```rust
use clap::Args;
use serde_json::Value;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct CreatePlaylistArgs {
    /// Raw JSON body (overrides individual flags)
    #[arg(long)]
    pub data: Option<String>,

    /// Playlist name
    #[arg(long)]
    pub name: Option<String>,

    /// Playlist description
    #[arg(long)]
    pub description: Option<String>,

    /// Make the playlist public
    #[arg(long)]
    pub public: bool,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &CreatePlaylistArgs) -> Result<(), CliError> {
    let urls = config::load_service_urls()?;
    let url = config::require_url(&urls, "api")?;
    let endpoint = format!("{url}/me/playlists");

    // Build body from --data or individual flags
    let body: Value = if let Some(ref raw) = args.data {
        serde_json::from_str(raw)
            .map_err(|e| CliError::Validation(format!("Invalid JSON in --data: {e}")))?
    } else {
        let mut obj = serde_json::Map::new();
        if let Some(ref name) = args.name {
            obj.insert("name".into(), Value::String(name.clone()));
        }
        if let Some(ref desc) = args.description {
            obj.insert("description".into(), Value::String(desc.clone()));
        }
        obj.insert("public".into(), Value::Bool(args.public));

        if !obj.contains_key("name") {
            return Err(CliError::Validation(
                "Provide --name or --data '{\"name\": \"...\"}'".into(),
            ));
        }
        Value::Object(obj)
    };

    let http = client::build_client()?;
    let resp = client::send_authenticated(&http, |token| {
        http.post(&endpoint).bearer_auth(token).json(&body)
    })
    .await?;

    let result = client::handle_api_response(resp).await?;
    let fmt = OutputFormat::from_str(&args.format);
    println!("{}", formatter::format_value(&result, &fmt));
    Ok(())
}
```

## Step 6: Register Your Commands

For each command file you created:

1. **Add the module** to `src/commands/mod.rs`:

```rust
pub mod playlists;
pub mod playlist;
pub mod player;
pub mod create_playlist;
```

2. **Add the variant** to the `Command` enum in `src/main.rs`:

```rust
#[derive(Subcommand, Debug)]
enum Command {
    // ... existing framework commands (auth, config, status, etc.) ...

    /// List your playlists
    Playlists(commands::playlists::PlaylistsArgs),

    /// Get playlist tracks
    Playlist(commands::playlist::PlaylistArgs),

    /// Control playback
    Player {
        #[command(subcommand)]
        command: commands::player::PlayerCommand,
    },

    /// Create a new playlist
    CreatePlaylist(commands::create_playlist::CreatePlaylistArgs),
}
```

3. **Add the dispatch arm** in the `match` block:

```rust
let result = match &cli.command {
    // ... existing arms ...
    Command::Playlists(args) => commands::playlists::handle(args).await,
    Command::Playlist(args) => commands::playlist::handle(args).await,
    Command::Player { command } => commands::player::handle(command).await,
    Command::CreatePlaylist(args) => commands::create_playlist::handle(args).await,
};
```

4. **Verify:**
```sh
cargo check
```

## Step 7: Create Your .env.example

Create a **`.env.example`** so users know which env vars are available:

```env
# spotify-cli environment overrides

# API token (skips login)
SPOTIFY_CLI_TOKEN=

# Override API base URL
SPOTIFY_CLI_API_URL=https://api.spotify.com/v1

# Project context (optional)
SPOTIFY_CLI_PROJECT_ID=
SPOTIFY_CLI_ENV_ID=
SPOTIFY_CLI_STAGE=
```

## Step 8: Build and Test

```sh
# Compile
cargo build

# Run tests
cargo test

# Check for warnings
cargo clippy -- -D warnings

# See your CLI in action
cargo run -- --help
cargo run -- player status --format table
```

## Step 9: Install

```sh
# Install to your PATH
cargo install --path .

# Now use it directly
spotify-cli --help
spotify-cli auth login
spotify-cli playlists --format table
```

---

## What to Keep, What to Replace

When forking nucleo, you **keep** all the framework infrastructure and **replace** only the domain-specific parts.

### Keep (don't touch these)

| Component | Why |
|-----------|-----|
| `src/error.rs` | Error handling with typed exit codes |
| `src/formatter.rs` | 6 output formats (JSON, table, YAML, CSV, IDs, Slack) |
| `src/client.rs` | HTTP client with retry, token refresh, 401 retry |
| `src/config.rs` | Layered config system with presets |
| `src/oauth2.rs` | OAuth2 Authorization Code + PKCE |
| `src/types/` | Auth, pagination, config types |
| `src/mcp/` | MCP server for Claude Desktop |
| `src/commands/auth.rs` | Login/logout/token |
| `src/commands/config_cmd.rs` | Config management |
| `src/commands/status.rs` | System status overview |
| `src/commands/plugins.rs` | Plugin lifecycle |
| `src/commands/setup.rs` | Interactive setup wizard |
| `src/commands/mcp_cmd.rs` | MCP server launcher |

### Replace

| Component | What to change |
|-----------|---------------|
| `src/consts.rs` | Your 4 identity constants |
| `Cargo.toml` | Package name, description, binary name |
| `src/main.rs` | CLI name, about text, Command enum, dispatch |
| `src/commands/ping.rs` | Delete (example command) |
| `src/commands/echo.rs` | Delete (example command) |
| `config.json` | Your API URLs and auth config |
| `.env.example` | Your env var prefix |
| `src/mcp/tools.rs` | Your MCP tools (optional) |

---

## Adding MCP Tools (Optional)

MCP tools let your CLI work with Claude Desktop. For each command, add a tool to `src/mcp/tools.rs`:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PlaylistsParams {
    /// Maximum number of playlists to return
    pub limit: Option<u32>,
}

#[tool(
    name = "spotify_playlists",
    description = "List the user's Spotify playlists"
)]
async fn tool_playlists(
    &self,
    Parameters(params): Parameters<PlaylistsParams>,
) -> String {
    let mut args = vec!["playlists".to_string(), "--format".to_string(), "json".to_string()];
    if let Some(limit) = params.limit {
        args.push("--limit".to_string());
        args.push(limit.to_string());
    }
    self.run_owned(&args).await
}
```

Then update the server info in `src/mcp/mod.rs` to match your CLI name.

Configure Claude Desktop to use your CLI:

```json
{
  "mcpServers": {
    "spotify-cli": {
      "command": "spotify-cli",
      "args": ["mcp"]
    }
  }
}
```

Or run the setup wizard:

```sh
spotify-cli setup --claude-desktop
```

---

## Common Patterns

### Pagination

Many APIs return paginated results. Add `--limit`, `--offset`, and `--all` flags:

```rust
#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, default_value = "20")]
    pub limit: u32,

    #[arg(long, default_value = "0")]
    pub offset: u32,

    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value = "json")]
    pub format: String,
}
```

When `--all` is set, loop through pages until the API returns an empty result or no `next` URL.

### Search

For search commands, add a required query argument:

```rust
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Type of item to search for
    #[arg(long, default_value = "track")]
    pub item_type: String,

    #[arg(long, default_value = "json")]
    pub format: String,
}
```

### Multiple Environments

If your API has staging/production environments, add multiple presets:

```json
{
  "presets": {
    "production": {
      "urls": { "api": "https://api.example.com/v1" },
      "auth_method": "oauth2",
      "oauth2": { "..." : "..." }
    },
    "staging": {
      "urls": { "api": "https://api.staging.example.com/v1" },
      "auth_method": "oauth2",
      "oauth2": { "..." : "..." }
    }
  }
}
```

Switch between them:

```sh
spotify-cli config env use staging
```

---

## The Fast Way: Claude Code

If you use [Claude Code](https://claude.com/claude-code), you can skip most of this guide.

### Install the nucleo skills

```sh
npx skills add mateonunez/nucleo
```

This installs all nucleo skills, including `/create-cli`. Browse them at [skills.sh/mateonunez/nucleo](https://skills.sh/mateonunez/nucleo).

### Run it

```
/create-cli
```

Claude will ask you 3 questions (CLI name, description, target API), then automatically:

1. Discover your API endpoints (from OpenAPI specs, documentation pages, or built-in profiles for 15+ popular APIs)
2. Configure authentication
3. Generate all command files with proper error handling
4. Create MCP tools for Claude Desktop
5. Write tests
6. Update the README
7. Verify the build compiles

It supports OpenAPI/Swagger specs, live documentation URLs (ReadMe, Redoc, Swagger UI, Postman collections), and well-known APIs (Spotify, GitHub, Stripe, Slack, and more).

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo check` fails after removing ping/echo | Make sure you removed all references from `mod.rs` and `main.rs` (both the enum variant and the match arm) |
| "Not authenticated" error | Run `spotify-cli auth login` or set `SPOTIFY_CLI_TOKEN` |
| "No 'api' URL configured" | Check `~/.config/spotify-cli/config.json` has the `api` URL |
| OAuth2 login opens browser but nothing happens | Verify your redirect URI is set to `http://127.0.0.1:8888/callback` in your OAuth2 app settings |
| Commands return 401 | Your token may have expired. Run `spotify-cli auth login` again |
| `cargo clippy` warnings | Fix all warnings before shipping — nucleo's CI runs clippy with `-D warnings` |

## Next Steps

- Add more commands as you discover new endpoints
- Create [plugins](../../plugins/) for features in other languages
- Add [templates](../../templates/) for project scaffolding
- Run [benchmarks](../../benchmarks/run.sh) to measure performance
- Set up CI/CD — nucleo's `.github/workflows/` are ready to use

---

*This guide covers the manual process. For the automated path, use Claude Code's `/create-cli` skill.*
