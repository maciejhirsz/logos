//! Simple 1D array language with sum and product operators
//! and predefined variables.
//!
//! Demonstrates the explicit lifetime attribute by storing borrows of
//! the variable environment within multiple lexers in different threads.
//!
//! Usage:
//!     cargo run --example array_language <path/to/file>
//!
//! Example:
//!     cargo run --example array_language examples/array_program.txt

/* ANCHOR: all */
use logos::{Lexer, Logos};

use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseIntError;
use std::path::Path;

/* ANCHOR: error_type */
/// Token error type, tied to the lifetime of the source.
#[derive(Default, Debug, Clone, PartialEq)]
enum LexingError<'s> {
    UnknownSymbol(&'s str),
    InvalidInteger {
        err: ParseIntError,
        source: &'s str,
    },
    UnknownVariable(&'s str),
    #[default]
    Other,
}
/* ANCHOR_END: error_type */

impl Display for LexingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSymbol(s) => write!(f, "unknown symbol `{s}`"),
            Self::InvalidInteger { err, source } => {
                write!(f, "int error in source `{source}`: {err}")
            }
            Self::UnknownVariable(s) => write!(f, "unknown variable `{s}`"),
            Self::Other => write!(f, "unknown error"),
        }
    }
}

/// Structure to map variable names to values.
type Environment = HashMap<String, Vec<i128>>;

/* ANCHOR: callbacks */
/// Parse lexer slice as an i128
fn number_callback<'s>(lex: &mut Lexer<'s, Token>) -> Result<i128, LexingError<'s>> {
    let source = lex.slice();
    let res = source.parse();
    res.map_err(|err| LexingError::InvalidInteger { err, source })
}

/// Look up the lexer slice in the variable environment,
/// yielding a borrow of the variable's value.
fn var_callback<'s, 'a>(lex: &mut Lexer<'s, Token<'a>>) -> Result<&'a [i128], LexingError<'s>> {
    match lex.extras.get(lex.slice()) {
        Some(arr) => Ok(arr.as_slice()),
        None => Err(LexingError::UnknownVariable(lex.slice())),
    }
}
/* ANCHOR_END: callbacks */

/* ANCHOR: tokens */
#[derive(Debug, Logos)]
#[logos(lifetime = none)]
#[logos(error(LexingError<'s>, |lex| LexingError::UnknownSymbol(lex.slice())))]
#[logos(extras = &'a Environment)]
#[logos(skip " +")]
enum Token<'a> {
    #[regex(r"\-?[0-9]+", number_callback)]
    Number(i128),
    #[regex(r"[[:alpha:]][[:alnum:]]*", var_callback)]
    Array(&'a [i128]),
    #[token("*")]
    Product,
    #[token("+")]
    Sum,
    #[token("~")]
    Reverse,
}
/* ANCHOR_END: tokens */

/* ANCHOR: evaluate */
/// Evaluate a sequence of tokens to produce an array.
fn evaluate(tokens: &[Token]) -> Vec<i128> {
    let mut accumulator = Vec::new();
    for tok in tokens {
        match *tok {
            Token::Number(n) => accumulator.push(n),
            Token::Array(arr) => accumulator.extend(arr),
            Token::Product => {
                let n = accumulator.drain(..).product();
                accumulator.push(n);
            }
            Token::Sum => {
                let n = accumulator.drain(..).sum();
                accumulator.push(n);
            }
            Token::Reverse => accumulator.reverse(),
        }
    }
    accumulator
}
/* ANCHOR_END: evaluate */

/* ANCHOR: lex_file */
/// Open the given file and lex each line, returning the results.
/// For each line, a lexer is created on a new thread.
/// The environment is shared between all lexers.
fn lex_file<'a>(path: &Path, env: &'a Environment) -> Vec<Result<Vec<Token<'a>>, String>> {
    let source = std::fs::read_to_string(path).expect("Failed to read file");
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for line in source.lines() {
            handles.push(s.spawn(|| {
                let lexer = Token::lexer_with_extras(line, env);
                // Convert the lexer errors to strings before returning,
                // because the source is scoped to this function.
                lexer.map(|res| res.map_err(|e| e.to_string())).collect()
            }));
        }
        handles.into_iter().flat_map(|h| h.join()).collect()
    })
}
/* ANCHOR_END: lex_file */

fn main() {
    let filename = std::env::args().nth(1).expect("Expected file argument");
    let env = Environment::from([
        ("NAT".to_owned(), (1..=10).collect()),
        ("EVEN".to_owned(), (2..=20).step_by(2).collect()),
        ("ODD".to_owned(), (1..=20).step_by(2).collect()),
        ("PRIME".to_owned(), vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]),
        ("FIB".to_owned(), vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]),
        ("POW2".to_owned(), (0..=10).map(|n| 2i128.pow(n)).collect()),
    ]);

    let results = lex_file(Path::new(&filename), &env);
    for res in &results {
        match res {
            Ok(tokens) => {
                let numbers = evaluate(tokens).into_iter().map(|n| n.to_string());
                println!("[{}]", numbers.collect::<Vec<_>>().join(", "));
            }
            Err(s) => eprintln!("{s}"),
        }
    }
}
/* ANCHOR_END: all */
