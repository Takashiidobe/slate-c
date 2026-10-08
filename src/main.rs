use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, input] if command == "run" => {
            let source = std::fs::read_to_string(input)?;
            println!("{}", slate_c::native::run(&source)?);
        }
        [command, input, output] if command == "wasm" || command == "object" => {
            let source = std::fs::read_to_string(input)?;
            let bytes = if command == "wasm" {
                slate_c::compile_wasm(&source)?
            } else {
                slate_c::native::compile_object(&source)?
            };
            std::fs::write(output, bytes)?;
        }
        _ => bail!("usage: slate-c run INPUT.c | slate-c {{wasm|object}} INPUT.c OUTPUT"),
    }
    Ok(())
}
