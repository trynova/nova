use clap::{Args, Parser as ClapParser, Subcommand, ValueEnum};
use nova_parser::{Lexer, Parser, Token};

/// A JavaScript engine
#[derive(Debug, ClapParser)] // requires `derive` feature
#[command(name = "nova")]
#[command(about = "A JavaScript engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(arg_required_else_help = true)]
    Tokenize {
        /// The path of the file to tokenize
        path: String,
    },

    #[command(arg_required_else_help = true)]
    Parse {
        /// The path of the file to parse
        path: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Command::Tokenize { path } => {
            let source = std::fs::read_to_string(path.as_str())?;
            let mut lex = Lexer::new(source.as_str());

            loop {
                lex.next();
                println!(
                    "{:?} '{}'{}",
                    lex.token,
                    &source[lex.start..lex.index],
                    if lex.has_newline_before {
                        " (has newline before)"
                    } else {
                        ""
                    }
                );
                if let Token::EOF = lex.token {
                    break;
                }
            }
        }
        Command::Parse { path } => {
            let source = std::fs::read_to_string(path.as_str())?;

            let mut parser = Parser::new(source.as_str());
            let scope = parser.parse_global_scope().unwrap();

            for node in scope.iter() {
                let Some(node) = parser.nodes.get(*node) else {
                    unreachable!()
                };
                println!("{:?}", node);
            }
        }
    }

    Ok(())
}
