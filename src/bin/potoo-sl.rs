use std::io::{self, Write};

use potoo_sl_interpreter::{Host, Interpreter, parse};

struct CliHost {
    parameters: Vec<String>,
}

impl Host for CliHost {
    fn input(&mut self) -> Result<String, String> {
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        Ok(line)
    }

    fn output(&mut self, text: &str) -> Result<(), String> {
        print!("{text}");
        io::stdout().flush().map_err(|error| error.to_string())
    }

    fn parameters(&self) -> Vec<String> {
        self.parameters.clone()
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| "usage: potoo-sl <source-file>".to_owned())?;
    let source = std::fs::read_to_string(&path).map_err(|error| format!("{path}: {error}"))?;
    let program = parse(&source).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    Interpreter::new(CliHost {
        parameters: args.collect(),
    })
    .run(&program)
    .map_err(|error| error.to_string())?;
    Ok(())
}
