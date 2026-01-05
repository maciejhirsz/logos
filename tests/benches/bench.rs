use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use logos::Logos;

#[derive(Debug, Clone, Copy, PartialEq, Logos)]
pub enum Token {
    #[regex(r"[ \n\t\f]", logos::skip)]
    InvalidToken,

    #[regex("[a-zA-Z_$][a-zA-Z0-9_$]*")]
    Identifier,

    #[regex(r#""([^"\\]|\\t|\\u|\\n|\\")*""#)]
    String,

    #[token("private")]
    Private,

    #[token("primitive")]
    Primitive,

    #[token("protected")]
    Protected,

    #[token("in")]
    In,

    #[token("instanceof")]
    Instanceof,

    #[token(".")]
    Accessor,

    #[token("...")]
    Ellipsis,

    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("+")]
    OpAddition,

    #[token("++")]
    OpIncrement,

    #[token("=")]
    OpAssign,

    #[token("==")]
    OpEquality,

    #[token("===")]
    OpStrictEquality,

    #[token("=>")]
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

static CANDIDATES: [(&str, &str); 3] = [
    ("identifiers", IDENTIFIERS),
    ("keywords_operators_and_punctators", SOURCE),
    ("strings", STRINGS),
];

#[allow(unused_must_use)]
fn iterate(s: &str) {
    let mut lex = Token::lexer(s);

    while let Some(token) = lex.next() {
        black_box(token);
    }
}

fn count_ok(s: &str) -> usize {
    Token::lexer(s).filter_map(|res| res.ok()).count()
}

fn bench_iterate(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate");

    for (name, source) in CANDIDATES {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(name, &source, |b, &s| b.iter(|| iterate(s)));
    }
}

fn bench_count_ok(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_ok");

    for (name, source) in CANDIDATES {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(name, &source, |b, &s| b.iter(|| count_ok(s)));
    }
}

criterion_group!(benches, bench_iterate, bench_count_ok);
criterion_main!(benches);
