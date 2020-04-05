#![feature(test)]
extern crate test;

use logos_derive::Logos;
use test::{black_box, Bencher};

#[derive(Debug, Clone, Copy, PartialEq, Logos)]
pub enum Token {
    #[error]
    InvalidToken,

    #[end]
    EndOfProgram,

    #[regex = "[a-zA-Z_$][a-zA-Z0-9_$]*"]
    Identifier,

    #[regex = r#""([^"\\]|\\t|\\u|\\n|\\")*""#]
    String,

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

static IDENTIFIERS: &str = "It was the year when they finally immanentized the Eschaton \
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


static STRINGS: &str = r#""tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree." "tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree." "tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree." "tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree.""#;

#[bench]
fn identifiers(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = IDENTIFIERS.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(IDENTIFIERS);

        while lex.token != None {
            lex.advance();
        }

        black_box(lex.token)
    });
}

#[bench]
fn keywords_operators_and_punctators(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(SOURCE);

        while lex.token != None {
            lex.advance();
        }

        black_box(lex.token)
    });
}

#[bench]
fn strings(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = STRINGS.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(STRINGS);

        while lex.token != None {
            lex.advance();
        }

        black_box(lex.token)
    });
}

#[bench]
fn x_identifiers_iter(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = IDENTIFIERS.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(IDENTIFIERS);

        while let Some(token) = lex.next() {
            black_box(token);
        }
    });
}