use slate_c::diagnostics::{CompilerError, Result};
use std::process::ExitCode;

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", slate_c::diagnostics::render(&error));
            ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, input] if command == "run" => {
            let source = std::fs::read_to_string(input)?;
            println!(
                "{}",
                slate_c::native::run(&source).map_err(|error| error.with_input_name(input))?
            );
        }
        [command, input, output] if command == "object" => {
            let source = std::fs::read_to_string(input)?;
            let bytes = slate_c::native::compile_object(&source)
                .map_err(|error| error.with_input_name(input))?;
            std::fs::write(output, bytes)?;
        }
        _ => {
            return Err(CompilerError::Configuration(
                "usage: slate-c run INPUT.c | slate-c object INPUT.c OUTPUT.o".into(),
            ));
        }
    }
    Ok(())
}
