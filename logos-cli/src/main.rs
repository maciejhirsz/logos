use std::{
    io::{self, Read},
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use clap::Parser;
use fs_err as fs;
use proc_macro2::{LexError, TokenStream};

/// Logos as a CLI!
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Input file to process
    #[clap(parse(from_os_str))]
    input: PathBuf,
    /// Path to write output. By default output is printed to stdout.
    #[clap(long, short, parse(from_os_str))]
    output: Option<PathBuf>,
    /// Checks whether the output file is up-to-date instead of writing to it. Requires --output to be specified.
    #[clap(long, requires = "output")]
    check: bool,
    /// Invokes `rustfmt` on the generated code. `rustfmt` must be in $PATH.
    #[clap(long)]
    format: bool,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let input = fs::read_to_string(args.input)?;
    let mut output = codegen(input).context("failed to run rustfmt")?;

    if args.format {
        output = rustfmt(output)?;
    }

    if let Some(output_path) = args.output {
        let changed = match fs::read_to_string(&output_path) {
            Ok(existing_output) => existing_output != output,
            Err(err) if err.kind() == io::ErrorKind::NotFound => true,
            Err(err) => return Err(err.into()),
        };

        if !changed {
            Ok(())
        } else if args.check {
            Err(anyhow::format_err!(
                "contents of {} differed from generated code",
                output_path.display()
            ))
        } else {
            fs::write(output_path, output)?;
            Ok(())
        }
    } else {
        println!("{}", output);
        Ok(())
    }
}

fn codegen(input: String) -> Result<String> {
    let input_tokens: TokenStream = input
        .parse()
        .map_err(|err: LexError| anyhow::Error::msg(err.to_string()))
        .context("failed to parse input as rust code")?;
    let output = logos_codegen::generate(input_tokens);
    Ok(output.to_string())
}

fn rustfmt(input: String) -> Result<String> {
    let command = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()?;
    io::Write::write_all(&mut command.stdin.unwrap(), input.as_bytes())?;

    let mut output = String::new();
    command.stdout.unwrap().read_to_string(&mut output)?;
    Ok(output)
}
