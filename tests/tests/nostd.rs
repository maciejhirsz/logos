//! This test ensures that logos can be used in no_std environments

#![no_std]

use logos_derive::Logos;

#[derive(Logos)]
enum _Token {
    #[regex("[0-9]+")]
    Number,
    #[token("+")]
    Plus
}
