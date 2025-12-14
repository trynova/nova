// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Formatting values and errors.

use nova_vm::{
    ecmascript::{Agent, JsResult, Value},
    engine::{Bindable, GcScope},
};
use oxc_diagnostics::OxcDiagnostic;

pub fn print_result(agent: &mut Agent, result: JsResult<Value>, verbose: bool, gc: GcScope) {
    match result {
        Ok(result) => {
            if verbose {
                println!("{result:?}");
            }
        }
        Err(error) => {
            eprintln!(
                "Uncaught exception: {}",
                error
                    .value()
                    .unbind()
                    .string_repr(agent, gc)
                    .as_wtf8(agent)
                    .to_string_lossy()
            );
            std::process::exit(1);
        }
    }
}

/// Exit the program with parse errors.
pub fn exit_with_parse_errors(errors: Vec<OxcDiagnostic>, source_path: &str, source: &str) -> ! {
    assert!(!errors.is_empty());

    // This seems to be needed for color and Unicode output.
    miette::set_hook(Box::new(|_| {
        Box::new(oxc_diagnostics::GraphicalReportHandler::new())
    }))
    .unwrap();

    // SAFETY: This function never returns, so `source`'s lifetime must last for
    // the duration of the program.
    let source: &'static str = unsafe { std::mem::transmute(source) };
    let named_source = miette::NamedSource::new(source_path, source);

    eprintln!("SyntaxError:");

    for error in errors {
        let report = error.with_source_code(named_source.clone());
        eprintln!("{report:?}");
    }

    std::process::exit(1);
}
