use clap::{Parser as ClapParser, Subcommand};
use nova_vm::ecmascript::{
    execution::{agent::Options, initialize_default_realm, Agent, DefaultHostHooks},
    scripts_and_modules::script::{parse_script, script_evaluation},
};
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
        #[arg(short, long)]
        verbose: bool,

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
        Command::Eval { verbose, path } => {
            let file = std::fs::read_to_string(path)?;
            let allocator = Default::default();

            let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
            initialize_default_realm(&mut agent);
            let realm = agent.current_realm_id();

            let script = parse_script(&allocator, file.into(), realm, None).unwrap();
            let result = script_evaluation(&mut agent, script).unwrap();

            if verbose {
                println!("{:?}", result);
            }
        }
    }

    Ok(())
}
