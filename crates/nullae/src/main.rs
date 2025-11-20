use nullae_core::handler;
use tracing_subscriber::{EnvFilter, fmt};

use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
#[command(
    about = "0ae: Some description",
    override_usage = r#"0ae [-v | --version]

    "#,
    long_about = r#"
        +-+-+-+
        |0|A|E|
        +-+-+-+

Some description
"#
)]
pub(crate) enum Commands {
    Discovery {
        #[arg(short, long, default_value_t = String::from("local"))]
        domain: String,
    },
    List,
    Delete {
        #[arg(long, short)]
        pattern: String,
    },
    Show {
        pattern: String,
    },
}

#[derive(Parser, Debug)]
#[command(name = "0ae")]
pub struct Cli {
    #[arg(short, long)]
    version: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
pub async fn main() {
    dotenvy::dotenv().ok();

    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    tracing::info!("0ae started");

    let args = Cli::parse();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    };

    let result = match args.command {
        Some(Commands::Discovery { domain }) => handler::discovery(domain).await,
        Some(Commands::List) => handler::list().await,
        Some(Commands::Delete { pattern }) => handler::delete(pattern).await,
        Some(Commands::Show { pattern }) => handler::show(pattern).await,
        None => Ok(()),
    };

    if let Err(err) = result {
        tracing::error!(?err);
    };
}
