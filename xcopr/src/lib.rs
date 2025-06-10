use clap::Subcommand;

pub mod parser;
pub mod dag;
pub mod exec;

#[derive(Debug, Clone)]
pub struct Config {
    pub subcommand: XcoprSubcommand,
    pub coprocess: String,
    pub stream: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum XcoprSubcommand {
    Filter {
        #[arg(short = 'c', long = "coprocess", action = clap::ArgAction::Append)]
        coprocesses: Vec<String>,
        #[arg(short = 's', long = "stream", action = clap::ArgAction::Append)]
        streams: Vec<String>,
    },
    Map {
        #[arg(short = 'c', long = "coprocess", action = clap::ArgAction::Append)]
        coprocesses: Vec<String>,
        #[arg(short = 's', long = "stream", action = clap::ArgAction::Append)]
        streams: Vec<String>,
    },
    Diagram {
        #[arg(short = 'c', long = "coprocess", action = clap::ArgAction::Append)]
        coprocesses: Vec<String>,
        #[arg(short = 's', long = "stream", action = clap::ArgAction::Append)]
        streams: Vec<String>,
    },
}