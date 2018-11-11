#![feature(test)]

extern crate test;
extern crate logos;
extern crate luther;
extern crate pest;
extern crate toolshed;
#[macro_use] extern crate logos_derive;
#[macro_use] extern crate luther_derive;
#[macro_use] extern crate pest_derive;

use test::Bencher;

#[derive(Parser)]
#[grammar = "../benches/pestbench.pest"]
pub struct BenchParser;

#[derive(Debug, Clone, Copy, PartialEq, Logos, Lexer)]
pub enum Token {
    #[error]
    InvalidToken,

    #[end]
    EndOfProgram,

    // Logos ignores white space by defaut, Luther needs a token here
    #[luther(regex="[ \n]+")]
    Whitespace,

    #[regex = "[a-zA-Z_$][a-zA-Z0-9_$]*"]
    #[luther(regex="[a-zA-Z_$][a-zA-Z0-9_$]*")]
    Identifier,

    #[token = "private"]
    #[luther(regex="private")]
    Private,

    #[token = "primitive"]
    #[luther(regex="primitive")]
    Primitive,

    #[token = "protected"]
    #[luther(regex="protected")]
    Protected,

    #[token = "in"]
    #[luther(regex="in")]
    In,

    #[token = "instanceof"]
    #[luther(regex="instanceof")]
    Instanceof,

    #[token = "."]
    #[luther(regex="\\.")]
    Accessor,

    #[token = "..."]
    #[luther(regex="\\.\\.\\.")]
    Ellipsis,

    #[token = "("]
    #[luther(regex="\\(")]
    ParenOpen,

    #[token = ")"]
    #[luther(regex="\\)")]
    ParenClose,

    #[token = "{"]
    #[luther(regex="\\{")]
    BraceOpen,

    #[token = "}"]
    #[luther(regex="\\}")]
    BraceClose,

    #[token = "+"]
    #[luther(regex="\\+")]
    OpAddition,

    #[token = "++"]
    #[luther(regex="\\+\\+")]
    OpIncrement,

    #[token = "="]
    #[luther(regex="=")]
    OpAssign,

    #[token = "=="]
    #[luther(regex="==")]
    OpEquality,

    #[token = "==="]
    #[luther(regex="===")]
    OpStrictEquality,

    #[token = "=>"]
    #[luther(regex="=>")]
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
";

#[bench]
fn logos(b: &mut Bencher) {
    use logos::Logos;

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(SOURCE);

        while lex.token != Token::EndOfProgram {
            lex.consume()
        }
    });
}

#[bench]
fn logos_nul_terminated(b: &mut Bencher) {
    use logos::Logos;
    use toolshed::Arena;

    let arena = Arena::new();
    let ptr = arena.alloc_str_with_nul(SOURCE);

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let mut lex = Token::lexer(ptr);

        while lex.token != Token::EndOfProgram {
            lex.consume()
        }
    });
}

#[bench]
fn pest(b: &mut Bencher) {
    use pest::Parser;

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let _ = BenchParser::parse(Rule::bench, SOURCE).unwrap();
    });
}

#[bench]
fn luther(b: &mut Bencher) {
    use luther::Lexer;
    use luther::spanned::SpannedStrIter;

    b.bytes = SOURCE.len() as u64;

    b.iter(|| {
        let source = SpannedStrIter::new(SOURCE);
        let mut _token;

        for t in Token::lexer(source) {
            _token = t.unwrap();
        }
    });
}
