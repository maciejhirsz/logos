#![feature(test)]

extern crate test;
extern crate logos;

#[cfg(feature = "nul_term_source")]
extern crate toolshed;

use test::Bencher;
use logos::Logos;

#[derive(Debug, Clone, Copy, PartialEq, Logos)]
pub enum Token {
    #[error]
    InvalidToken,

    #[end]
    EndOfProgram,

    #[regex = "[a-zA-Z_$][a-zA-Z0-9_$]*"]
    Identifier,

    #[token = "private"]
    Private,

    #[token = "primitive"]
    Primitive,

    #[token = "protected"]
    Protected,

    #[token = "in"]
    In,

    #[token = "instanceof"]
    Instanceof,

    #[token = "."]
    Accessor,

    #[token = "..."]
    Ellipsis,

    #[token = "("]
    ParenOpen,

    #[token = ")"]
    ParenClose,

    #[token = "{"]
    BraceOpen,

    #[token = "}"]
    BraceClose,

    #[token = "+"]
    OpAddition,

    #[token = "++"]
    OpIncrement,

    #[token = "="]
    OpAssign,

    #[token = "=="]
    OpEquality,

    #[token = "==="]
    OpStrictEquality,

    #[token = "=>"]
    FatArrow,
}

static SOURCE: &str = "
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
foobar(protected primitive private instanceof in) { + ++ = == === => }
";


#[bench]
fn identifiers(b: &mut Bencher) {
    use logos::Logos;

    let source = "It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton \
                  It was the year when they finally immanentized the Eschaton";

    b.bytes = source.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(source);

        while lex.token != Token::EndOfProgram {
            lex.advance()
        }
    });
}

#[bench]
fn logos(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(SOURCE);

        while lex.token != Token::EndOfProgram {
            lex.advance()
        }
    });
}

#[cfg(feature = "nul_term_source")]
#[bench]
fn logos_nul_terminated(b: &mut Bencher) {
    use logos::Logos;
    use toolshed::Arena;

    let arena = Arena::new();
    let nts = arena.alloc_nul_term_str(SOURCE);

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(nts);

        while lex.token != Token::EndOfProgram {
            lex.advance()
        }
    });
}
