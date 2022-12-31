use clap::{Parser as ClapParser, Subcommand};
use codespan_reporting::{
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use nova_checker::{Checker, Env};
use nova_parser::parser::Parser;

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
    /// Type checks the given file.
    #[command(arg_required_else_help = true)]
    Check {
        path: String,
        #[arg(short, long, default_value_t = false)]
        debug_scope: bool,
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
            } else {
                eprintln!("error: {}", parser.error);
                eprintln!("{}:{}", &path, parser.lex.index);
                std::process::exit(1);
            }
        }
        Commands::Check { path, debug_scope } => {
            let input = std::fs::read_to_string(&path)?;

            let mut parser = Parser::new(&input);

            let Ok(nodes) = parser.parse_global_scope() else {
                eprintln!("error: {}", parser.error);
                eprintln!("{}:{}", &path, parser.lex.index);
                std::process::exit(1);
            };

            let mut checker = Checker::new(&input);
            let mut env = Env::new();

            let mut files = SimpleFiles::new();

            let file_id = files.add(&path, &input);

            let Ok(_) = checker.check_scope(&mut env, &nodes) else {
				let writer = StandardStream::stderr(ColorChoice::Always);
				let config = codespan_reporting::term::Config::default();

				term::emit(&mut writer.lock(), &config, &files, &checker.diagnostics[0])?;
				std::process::exit(1);
			};

            if debug_scope {
                println!("{:#?}", env.entries);
            }
        }
    }

    Ok(())
}
