use clap::{Args, Parser as ClapParser, Subcommand, ValueEnum};
use parser::Parser;
use tokenizer::{Token, TokenStream};

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
            let mut stream = TokenStream::new(source.as_str());

            loop {
                stream.next();
                println!(
                    "{:?} '{}'{}",
                    stream.token,
                    &source[stream.start..stream.index],
                    if stream.has_newline_before {
                        " (has newline before)"
                    } else {
                        ""
                    }
                );
                if let Token::EOF = stream.token {
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
