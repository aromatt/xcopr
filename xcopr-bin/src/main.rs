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
            // Support different patterns:
            // 1. Single -c and -s: simple coprocess + template
            // 2. Only -s with inline commands: template-only mode
            // 3. Multiple -c and -s: full DAG mode (TODO)
            
            if streams.len() != 1 {
                eprintln!("Currently only supports exactly one -s flag");
                std::process::exit(1);
            }
            
            let stream_template = &streams[0];
            
            // Parse the stream template to find token references
            let (tokenized_template, stream_defs) = xcopr::parser::parse_tokens_with_template(stream_template);
            
            if coprocesses.is_empty() {
                // Template-only mode: all processing happens via inline commands
                xcopr::exec::execute_template_only(&tokenized_template, &stream_defs).await?;
            } else if coprocesses.len() == 1 {
                // Simple mode: one coprocess + template
                let coprocess_cmd = &coprocesses[0];
                xcopr::exec::execute_map_simple(coprocess_cmd, &tokenized_template, &stream_defs).await?;
            } else {
                // Multiple coprocesses mode
                xcopr::exec::execute_map_multiple(&coprocesses, &tokenized_template, &stream_defs).await?;
            }
        },
        XcoprSubcommand::Diagram { coprocesses, streams } => {
            println!("Diagram mode not yet updated for new CLI structure");
            println!("Coprocesses: {:?}", coprocesses);
            println!("Streams: {:?}", streams);
        },
    }
    
    Ok(())
}