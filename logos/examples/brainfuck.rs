//! Brainfuck interpreter written in Rust, using Logos.
//!
//! Brainfuck is an esoteric programming language that only
//! uses 8 single-character commands:
//! - '>';
//! - '<';
//! - '+';
//! - '-';
//! - '.';
//! - ',';
//! - '[';
//! - and ']'.
//!
//! Despite being very hard to use in practive, this makes
//! this language very simple to interpet. The following code
//! defines an [`execute`] function that runs Brainfuck code.
//!
//! Logos is used here to directly transform the code stream
//! into meaningful `Op` operations (or commands).
//! Errors, i.e., unknown tokens, are discarded using `filter_map`.
//!
//! The [`main`] function runs an "Hello Worlds!" program.
//!
//! More details can be found on Wikipedia:
//! https://en.wikipedia.org/wiki/Brainfuck.

use logos::Logos;
use std::collections::HashMap;
use std::io::{self, Read};

/// Each [`Op`] variant is a single character.
#[derive(Debug, Logos)]
enum Op {
    #[token(">")]
    IncPointer,
    #[token("<")]
    DecPointer,
    #[token("+")]
    IncData,
    #[token("-")]
    DecData,
    #[token(".")]
    OutData,
    #[token(",")]
    InpData,
    #[token("[")]
    CondJumpForward,
    #[token("]")]
    CondJumpBackward,
}

/// Print one byte to the terminal.
#[inline(always)]
fn print_byte(byte: u8) {
    print!("{}", byte as char);
}

/// Read one byte from the terminal.
#[inline(always)]
fn read_byte() -> u8 {
    let mut input = [0u8; 1];
    io::stdin()
        .read_exact(&mut input)
        .expect("An error occurred while reading byte!");
    input[0]
}

/// Execute Brainfuck code from a string slice.
fn execute(code: &str) {
    let operations: Vec<_> = Op::lexer(code).filter_map(|op| op.ok()).collect();
    let mut data = [0u8; 30_000]; // Minimum recommended size
    let mut pointer: usize = 0;
    let mut i: usize = 0;
    let len = operations.len();

    // We pre-process matching jump commands, and we create
    // a mapping between them.
    //
    // This is the only portion of code that could panic (or the data allocated being not large
    // enough).
    let mut queue = Vec::new();
    let mut pairs = HashMap::new();
    let mut pairs_reverse = HashMap::new();

    for (i, op) in operations.iter().enumerate() {
        match op {
            Op::CondJumpForward => queue.push(i),
            Op::CondJumpBackward => {
                if let Some(start) = queue.pop() {
                    pairs.insert(start, i);
                    pairs_reverse.insert(i, start);
                } else {
                    panic!(
                        "Unexpected conditional backward jump at position {}, does not match any '['",
                        i
                    );
                }
            }
            _ => (),
        }
    }

    if !queue.is_empty() {
        panic!("Unmatched conditional forward jump at positons {:?}, expecting a closing ']' for each of them", queue);
    }

    // True program execution.
    loop {
        match operations[i] {
            Op::IncPointer => pointer += 1,
            Op::DecPointer => pointer -= 1,
            Op::IncData => data[pointer] = data[pointer].wrapping_add(1),
            Op::DecData => data[pointer] = data[pointer].wrapping_sub(1),
            Op::OutData => print_byte(data[pointer]),
            Op::InpData => data[pointer] = read_byte(),
            Op::CondJumpForward => {
                if data[pointer] == 0 {
                    // Skip until matching end.
                    i = *pairs.get(&i).unwrap();
                }
            }
            Op::CondJumpBackward => {
                if data[pointer] != 0 {
                    // Go back to matching start.
                    i = *pairs_reverse.get(&i).unwrap();
                }
            }
        }
        i += 1;

        if i >= len {
            break;
        }
    }
}

fn main() {
    /*
     * Hellow World! program from Wikipedia
     * https://en.wikipedia.org/wiki/Brainfuck
     */
    let code = r#"
        [ This program prints "Hello World!" and a newline to the screen, its
          length is 106 active command characters. [It is not the shortest.]
        
          This loop is an "initial comment loop", a simple way of adding a comment
          to a BF program such that you don't have to worry about any command
          characters. Any ".", ",", "+", "-", "<" and ">" characters are simply
          ignored, the "[" and "]" characters just have to be balanced. This
          loop and the commands it contains are ignored because the current cell
          defaults to a value of 0; the 0 value causes this loop to be skipped.
        ]
        ++++++++               Set Cell #0 to 8
        [
            >++++               Add 4 to Cell #1; this will always set Cell #1 to 4
            [                   as the cell will be cleared by the loop
                >++             Add 2 to Cell #2
                >+++            Add 3 to Cell #3
                >+++            Add 3 to Cell #4
                >+              Add 1 to Cell #5
                <<<<-           Decrement the loop counter in Cell #1
            ]                   Loop until Cell #1 is zero; number of iterations is 4
            >+                  Add 1 to Cell #2
            >+                  Add 1 to Cell #3
            >-                  Subtract 1 from Cell #4
            >>+                 Add 1 to Cell #6
            [<]                 Move back to the first zero cell you find; this will
                                be Cell #1 which was cleared by the previous loop
            <-                  Decrement the loop Counter in Cell #0
        ]                       Loop until Cell #0 is zero; number of iterations is 8
        
        The result of this is:
        Cell no :   0   1   2   3   4   5   6
        Contents:   0   0  72 104  88  32   8
        Pointer :   ^
        
        >>.                     Cell #2 has value 72 which is 'H'
        >---.                   Subtract 3 from Cell #3 to get 101 which is 'e'
        +++++++..+++.           Likewise for 'llo' from Cell #3
        >>.                     Cell #5 is 32 for the space
        <-.                     Subtract 1 from Cell #4 for 87 to give a 'W'
        <.                      Cell #3 was set to 'o' from the end of 'Hello'
        +++.------.--------.    Cell #3 for 'rl' and 'd'
        >>+.                    Add 1 to Cell #5 gives us an exclamation point
        >++.                    And finally a newline from Cell #6        
    "#;

    execute(code);
}
