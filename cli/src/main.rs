use clap::{Parser as ClapParser, Subcommand};
use parser::parser::Parser;

#[derive(Debug, ClapParser)]
#[command(name = "nova")]
#[command(about = "A JavaScript runtime", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Prints out the AST for a given source file.
    #[command(arg_required_else_help = true)]
    Stat {
        path: String,
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Stat { path, verbose } => {
            let input = std::fs::read_to_string(&path)?;

            let mut parser = Parser::new(&input);

            if let Ok(nodes) = parser.parse_global_scope() {
                if verbose {
                    println!("{nodes:#?}");
                } else {
                    println!("{nodes:?}");
                }
            } else if verbose {
                eprintln!("error: {}", parser.error);
                eprintln!("{}:{}", &path, parser.lex.index);
            }
        }
    }

    Ok(())
}
