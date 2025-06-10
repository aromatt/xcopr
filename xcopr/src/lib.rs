use clap::Subcommand;

pub mod parser;
pub mod dag;

#[derive(Debug, Clone)]
pub struct Config {
    pub subcommand: XcoprSubcommand,
    pub coprocess: String,
    pub stream: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum XcoprSubcommand {
    Filter {
        #[arg(short = 'c', long = "coprocess")]
        coprocess: String,
        #[arg(short = 's', long = "stream")]
        stream: String,
    },
    Map {
        #[arg(short = 'c', long = "coprocess")]
        coprocess: String,
        #[arg(short = 's', long = "stream")]
        stream: String,
    },
    Diagram {
        #[arg(short = 'c', long = "coprocess")]
        coprocess: String,
        #[arg(short = 's', long = "stream")]
        stream: String,
    },
}