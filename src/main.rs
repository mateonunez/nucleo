mod client;
mod commands;
mod config;
mod consts;
mod error;
mod formatter;
mod mcp;
mod types;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use error::print_error_json;
use std::io;

#[derive(Parser, Debug)]
#[command(
    name = "nucleo",
    version,
    about = "nucleo — the nucleus of your CLI",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        command: commands::auth::AuthCommand,
    },
    /// Manage CLI configuration
    Config {
        #[command(subcommand)]
        command: commands::config_cmd::ConfigCommand,
    },
    /// Show overall status: auth, project context, config, and CLI version
    Status {
        /// Output format: text (default), json, yaml, csv
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Ping a configured service URL to check connectivity
    Ping(commands::ping::PingArgs),
    /// POST data to an echo service (demonstrates authenticated requests)
    Echo(commands::echo::EchoArgs),
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Manage and run plugins (language-agnostic extensions)
    Plugins {
        #[command(subcommand)]
        command: commands::plugins::PluginsCommand,
    },
    /// Start MCP server on stdio (used by Claude Desktop)
    Mcp,
    /// Interactive setup wizard: credentials, environment, Claude Desktop config
    Setup(commands::setup::SetupArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle sync commands before entering the async dispatch block.
    if let Command::Completions { shell } = &cli.command {
        generate(*shell, &mut Cli::command(), "nucleo", &mut io::stdout());
        return;
    }

    let result = match &cli.command {
        Command::Auth { command } => commands::auth::handle(command).await,
        Command::Config { command } => commands::config_cmd::handle(command).await,
        Command::Status { format } => commands::status::handle(format).await,
        Command::Ping(args) => commands::ping::handle(args).await,
        Command::Echo(args) => commands::echo::handle(args).await,
        Command::Plugins { command } => commands::plugins::handle(command).await,
        Command::Mcp => commands::mcp_cmd::handle().await,
        Command::Setup(args) => commands::setup::handle(args).await,
        // Handled above before this block; unreachable at runtime.
        Command::Completions { .. } => unreachable!(),
    };

    if let Err(err) = result {
        print_error_json(&err);
        std::process::exit(err.exit_code());
    }
}
