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
use logos::Logos;

use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::num::ParseIntError;
use std::path::Path;

/* ANCHOR: error_type */
/// Token error type, tied to the lifetime of the source.
#[derive(Debug, Clone, PartialEq)]
enum LexingError<'s> {
    UnknownSymbol(&'s str),
    InvalidInteger { err: ParseIntError, source: &'s str },
    UnknownVariable(&'s str),
}
/* ANCHOR_END: error_type */

impl Default for LexingError<'_> {
    fn default() -> Self {
        Self::UnknownSymbol("")
    }
}

impl Display for LexingError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSymbol(s) => write!(f, "unknown symbol `{s}`"),
            Self::InvalidInteger { err, source } => {
                write!(f, "int error in source `{source}`: {err}")
            }
            Self::UnknownVariable(s) => write!(f, "unknown variable `{s}`"),
        }
    }
}

/// Structure to map variable names to values.
type Environment = HashMap<String, Vec<i128>>;

fn number_callback<'s>(lex: &mut logos::Lexer<'s, Token>) -> Result<i128, LexingError<'s>> {
    let source = lex.slice();
    let res = source.parse();
    res.map_err(|err| LexingError::InvalidInteger { err, source })
}

fn var_callback<'s, 'a>(
    lex: &mut logos::Lexer<'s, Token<'a>>,
) -> Result<&'a [i128], LexingError<'s>> {
    match lex.extras.get(lex.slice()) {
        Some(arr) => Ok(arr.as_slice()),
        None => Err(LexingError::UnknownVariable(lex.slice())),
    }
}

/* ANCHOR: tokens */
#[derive(Debug, Logos, PartialEq)]
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
}
/* ANCHOR_END: tokens */

/* ANCHOR: evaluate */
/// Evaluate a sequence of tokens to produce an array.
fn evaluate(tokens: &[Token]) -> Vec<i128> {
    let mut out = Vec::new();
    for tok in tokens {
        match *tok {
            Token::Number(n) => out.push(n),
            Token::Array(arr) => out.extend(arr),
            Token::Product => {
                let n = out.drain(..).product();
                out.push(n);
            }
            Token::Sum => {
                let n = out.drain(..).sum();
                out.push(n);
            }
        }
    }
    out
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
                Token::lexer_with_extras(line, env)
                    .map(|res| res.map_err(|e| e.to_string()))
                    .collect()
            }));
        }
        handles.into_iter().flat_map(|h| h.join()).collect()
    })
}
/* ANCHOR_END: lex_file */

/// Simple pseudorandom number generator.
struct Minstd(u64);

impl Minstd {
    const M: u64 = 2u64.pow(31) - 1;
    const A: u64 = 7u64.pow(5);

    /// Generate `count` number of pseudorandom integers
    /// between `low` and `high`, inclusive.
    fn next(&mut self, count: usize, low: i128, high: i128) -> Vec<i128> {
        std::iter::from_fn(|| {
            self.0 = (self.0 * Self::A) % Self::M;
            Some(self.0 as i128 % (high + 1 - low) + low)
        })
        .take(count)
        .collect()
    }
}

fn main() {
    let filename = std::env::args().nth(1).expect("Expected file argument");

    // Seed PRNG with hash of filename.
    let mut minstd = Minstd({
        let mut hasher = DefaultHasher::new();
        filename.hash(&mut hasher);
        hasher.finish() % Minstd::M
    });

    // Create environment with variables A0 to A9.
    // Each is an array of 3 random integers (range -9..=9).
    let env = Environment::from_iter((0..10).map(|n| (format!("A{n}"), minstd.next(3, -9, 9))));

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
