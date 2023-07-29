use clap::{Parser as ClapParser, Subcommand};
use oxc_parser::Parser;
use oxc_span::SourceType;

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
    /// Parses a file and logs out the AST
    Parse {
        /// The path of the file to parse
        path: String,
    },

    /// Evaluates a file
    Eval {
        /// The file to evaluate
        path: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Command::Parse { path } => {
            let file = std::fs::read_to_string(path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            println!("{:?}", result.program);
        }
        Command::Eval { path } => {
            let file = std::fs::read_to_string(path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            // let mut vm = VM {
            //     source: &file,
            //     pc: 0,
            //     instructions: Vec::new(),
            //     heap: Heap::new(),
            // };

            // vm.load_program(result.program);
            // println!("{:?}", vm.instructions);
            // vm.interpret();
        }
    }

    Ok(())
}
