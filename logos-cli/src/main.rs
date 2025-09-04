use std::{
    fmt::Write,
    io,
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
            Ok(existing_output) => !eq_ignore_newlines(&existing_output, &output),
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
        println!("{output}");
        Ok(())
    }
}

fn codegen(input: String) -> Result<String> {
    let input_tokens: TokenStream = input
        .parse()
        .map_err(|err: LexError| anyhow::Error::msg(err.to_string()))
        .context("failed to parse input as rust code")?;

    let mut output = String::new();
    write!(
        output,
        "{}",
        logos_codegen::strip_attributes(input_tokens.clone())
    )?;
    write!(output, "{}", logos_codegen::generate(input_tokens))?;
    Ok(output)
}

fn rustfmt(input: String) -> Result<String> {
    let mut command = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()?;
    io::Write::write_all(&mut command.stdin.take().unwrap(), input.as_bytes())?;
    let output = command.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("rustfmt returned unsuccessful exit code");
    }

    String::from_utf8(output.stdout).context("failed to parse rustfmt output as utf-8")
}

fn eq_ignore_newlines(lhs: &str, rhs: &str) -> bool {
    lhs.lines().eq(rhs.lines())
}
