//! Variant of the JSON parser example, parsing from [`Read`] and allowing larger than memeory files.
//!
//! Usage:
//!     cargo run --example json-reader <path/to/file>
//!
//! Example:
//!     cargo run --example json-reader examples/example.json

use logos::{Lexer, Logos, Span};

use std::cmp::min;
use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::{env, fs, str};

const MAX_ALLOWED_BUFFER_LENGTH: usize = 4 * 1024 * 1024; // Maximal buffering of the lexer
const READ_AT_LEAST_BYTES: usize = 4096; // Minimal number of bytes to read from the file each time

/// All meaningful JSON tokens.
///
/// > NOTE: regexes for [`Token::Number`] and [`Token::String`] may not
/// > catch all possible values, especially for strings. If you find
/// > errors, please report them so that we can improve the regex.
#[expect(dead_code)]
#[derive(Debug, Logos)]
#[logos(skip r"[ \t\r\n\f]+", utf8 = false)]
enum Token<'source> {
    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Bool(bool),

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("null")]
    Null,

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| str::from_utf8(lex.slice()).unwrap().parse::<f64>().unwrap()
    )]
    Number(f64),

    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| str::from_utf8(lex.slice()).unwrap()
    )]
    String(&'source str),
}

struct BufferedLexer {
    buffer: Vec<u8>,
    start_buffer_offset: usize,
    end_buffer_offset: usize,
    offset_of_buffer_in_file: usize,
    last_token_size: usize,
    is_end: bool,
}

impl BufferedLexer {
    fn new() -> Self {
        Self {
            buffer: vec![0; READ_AT_LEAST_BYTES],
            start_buffer_offset: 0,
            end_buffer_offset: 0,
            offset_of_buffer_in_file: 0,
            last_token_size: 0,
            is_end: false,
        }
    }

    fn next(&mut self) -> Option<Result<Token<'_>, ()>> {
        // We build a lexer on the valid part of the buffer
        let valid_buffer_slice = &self.buffer[self.start_buffer_offset..self.end_buffer_offset];
        let mut lexer = if self.is_end {
            Lexer::new(valid_buffer_slice)
        } else {
            Lexer::new_prefix(valid_buffer_slice)
        };

        // We lex the next token
        let token = lexer.next()?;
        // We bump the offsets
        let span = lexer.span();
        self.last_token_size = span.end - span.start;
        self.start_buffer_offset += span.end;
        Some(token)
    }

    fn span(&self) -> Span {
        self.offset_of_buffer_in_file + self.start_buffer_offset - self.last_token_size
            ..self.offset_of_buffer_in_file + self.start_buffer_offset
    }

    fn fill_from_read<R: Read>(&mut self, mut reader: R) -> Result<(), Error> {
        // First, we shift the buffer to avoid always increasing it
        if self.start_buffer_offset > 0 {
            self.buffer
                .copy_within(self.start_buffer_offset..self.end_buffer_offset, 0);
            self.offset_of_buffer_in_file += self.start_buffer_offset;
            self.end_buffer_offset -= self.start_buffer_offset;
            self.start_buffer_offset = 0;
        }

        // If we don't have enough space in the buffer, we increase it
        if self.buffer.len() - self.end_buffer_offset < READ_AT_LEAST_BYTES {
            if self.buffer.len() == MAX_ALLOWED_BUFFER_LENGTH {
                return Err(Error::new(
                    ErrorKind::OutOfMemory,
                    "The lexer needs a too large buffer to parse the next token.",
                ));
            }
            self.buffer
                .resize(min(self.buffer.len() * 2, MAX_ALLOWED_BUFFER_LENGTH), 0);
        }

        // We do the read
        let read = reader.read(&mut self.buffer[self.end_buffer_offset..])?;
        self.end_buffer_offset += read;
        self.is_end = read == 0;
        Ok(())
    }

    fn is_end(&self) -> bool {
        self.is_end
    }
}

fn main() {
    let filename = env::args().nth(1).expect("Expected file argument");
    let mut file = File::open(&filename).expect("Failed to read file");
    let mut colors = ColorGenerator::new();

    let mut lexer = BufferedLexer::new();
    loop {
        // We read as many tokens as possible
        while let Some(token) = lexer.next() {
            match token {
                Ok(token) => println!("{token:?}"),
                Err(()) => {
                    let a = colors.next();
                    Report::build(ReportKind::Error, &filename, 12)
                        .with_message("Invalid JSON")
                        .with_label(
                            Label::new((&filename, lexer.span()))
                                .with_message("Unexpected token")
                                .with_color(a),
                        )
                        .finish()
                        .eprint((
                            &filename,
                            Source::from(fs::read_to_string(&filename).unwrap()),
                        ))
                        .unwrap();
                }
            }
        }
        if lexer.is_end() {
            break; // We are done parsing
        }
        // We add extra data to the lexer buffer to keep running
        lexer.fill_from_read(&mut file).expect("Failed to read file");
    }
}
