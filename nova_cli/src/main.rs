// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
mod helper;
mod theme;

use clap::{Parser as ClapParser, Subcommand};
use cliclack::{input, intro, set_theme};
use helper::{exit_with_parse_errors, CliRunner};
use nova_vm::ecmascript::types::Value;
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;
use theme::DefaultTheme;

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

        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Runs the REPL
    Repl {},
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Command::Parse { path } => {
            let file = std::fs::read_to_string(&path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            if !result.errors.is_empty() {
                exit_with_parse_errors(result.errors, &path, &file);
            }

            let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new(&file, source_type)
                .with_check_syntax_error(true)
                .build(&result.program);

            if !errors.is_empty() {
                exit_with_parse_errors(result.errors, &path, &file);
            }

            println!("{:?}", result.program);
        }
        Command::Eval {
            verbose,
            no_strict,
            paths,
        } => {
            let mut cli_runner = CliRunner::new(verbose);

            // `final_result` will always be overwritten in the paths loop, but
            // we populate it with a dummy value here so rustc won't complain.
            let mut final_result = Ok(Value::Undefined);

            assert!(!paths.is_empty());
            for path in paths {
                let file = std::fs::read_to_string(&path)?;
                final_result = cli_runner.run_script_and_microtasks(file.into(), &path, no_strict);
                if final_result.is_err() {
                    break;
                }
            }

            match final_result {
                Ok(result) => {
                    if verbose {
                        println!("{:?}", result);
                    }
                }
                Err(error) => {
                    eprintln!(
                        "Uncaught exception: {}",
                        error
                            .value()
                            .string_repr(cli_runner.agent())
                            .as_str(cli_runner.agent())
                    );
                    std::process::exit(1);
                }
            }
            std::process::exit(0);
        }
        Command::Repl {} => {
            let mut cli_runner = CliRunner::new(false);

            set_theme(DefaultTheme);
            println!("\n\n");
            let mut placeholder = "Enter a line of Javascript".to_string();

            loop {
                intro("Nova Repl (type exit or ctrl+c to exit)")?;
                let input: String = input("").placeholder(&placeholder).interact()?;

                if input.matches("exit").count() == 1 {
                    std::process::exit(0);
                }
                placeholder = input.to_string();

                match cli_runner.run_script_and_microtasks(input.into(), "<stdin>", false) {
                    Ok(result) => {
                        println!("{:?}\n", result);
                    }
                    Err(error) => {
                        eprintln!(
                            "Uncaught exception: {}",
                            error
                                .value()
                                .string_repr(cli_runner.agent())
                                .as_str(cli_runner.agent())
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
