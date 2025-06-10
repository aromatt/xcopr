use clap::Parser;
use xcopr::XcoprSubcommand;

mod commands;

#[derive(Parser)]
#[command(name = "xcopr")]
#[command(about = "A CLI tool for ergonomic coprocessing in Unix pipelines")]
struct Cli {
    #[command(subcommand)]
    command: XcoprSubcommand,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match &cli.command {
        XcoprSubcommand::Filter { coprocesses, streams } => {
            println!("Filter mode not yet implemented with new CLI structure");
            println!("Coprocesses: {:?}", coprocesses);
            println!("Streams: {:?}", streams);
        },
        XcoprSubcommand::Map { coprocesses, streams } => {
            // For now, handle simple case: one coprocess, one stream template
            if coprocesses.len() != 1 || streams.len() != 1 {
                eprintln!("Currently only supports exactly one -c and one -s flag");
                std::process::exit(1);
            }
            
            let coprocess_cmd = &coprocesses[0];
            let stream_template = &streams[0];
            
            // Parse the stream template to find token references
            let (tokenized_template, stream_defs) = xcopr::parser::parse_tokens_with_template(stream_template);
            
            // Execute: stdin → coprocess → substitute into stream template → stdout
            xcopr::exec::execute_map_simple(coprocess_cmd, &tokenized_template, &stream_defs).await?;
        },
        XcoprSubcommand::Diagram { coprocesses, streams } => {
            println!("Diagram mode not yet updated for new CLI structure");
            println!("Coprocesses: {:?}", coprocesses);
            println!("Streams: {:?}", streams);
        },
    }
    
    Ok(())
}