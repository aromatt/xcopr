use clap::Parser;
use xcopr::{Config, XcoprSubcommand};

#[derive(Parser)]
#[command(name = "xcopr")]
#[command(about = "A CLI tool for ergonomic coprocessing in Unix pipelines")]
struct Cli {
    #[command(subcommand)]
    command: XcoprSubcommand,
}

fn main() {
    let cli = Cli::parse();
    
    let config = match &cli.command {
        XcoprSubcommand::Filter { coprocess, stream } => Config {
            subcommand: cli.command.clone(),
            coprocess: coprocess.clone(),
            stream: stream.clone(),
        },
        XcoprSubcommand::Map { coprocess, stream } => Config {
            subcommand: cli.command.clone(),
            coprocess: coprocess.clone(),
            stream: stream.clone(),
        },
        XcoprSubcommand::Diagram { coprocess, stream } => Config {
            subcommand: cli.command.clone(),
            coprocess: coprocess.clone(),
            stream: stream.clone(),
        },
    };
    
    dbg!(config);
}