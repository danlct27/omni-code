use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omni-code", about = "Unified AI coding proxy")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the proxy server
    Proxy {
        /// Port to listen on
        #[arg(long, default_value = "8787")]
        port: u16,
        /// Path to config file
        #[arg(long, default_value = "~/.omni-code/config.toml")]
        config: String,
    },
    /// Show usage statistics
    Stats,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Proxy { port, .. } => {
            println!("Starting omni-code proxy on port {port}...");
        }
        Commands::Stats => {
            println!("Coming soon");
        }
    }
}
