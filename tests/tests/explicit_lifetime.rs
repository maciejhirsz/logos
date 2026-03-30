use logos::Filter;
use logos::Lexer;
use logos::Logos;

mod static_concrete_type {
    use super::*;

    #[derive(Logos, PartialEq, Debug)]
    #[logos(lifetime = none)]
    #[logos(type T = &'static str)]
    enum Token<T> {
        #[token("fizz", |_| "fizz")]
        #[token("buzz", |_| "buzz")]
        Value(T),
    }

    #[test]
    fn test() {
        let mut source = String::new();
        for n in 7..19 {
            if n % 3 == 0 {
                source += "fizz";
            }
            if n % 5 == 0 {
                source += "buzz";
            }
        }

        let mut lexer = <Token<&'static str> as Logos>::lexer(&source);

        assert_eq!(lexer.next(), Some(Ok(Token::Value("fizz"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Value("buzz"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Value("fizz"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Value("fizz"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Value("buzz"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Value("fizz"))));
        assert_eq!(lexer.next(), None);
    }
}

mod lifetime_none {
    use super::*;

    #[derive(Logos, PartialEq, Debug)]
    #[logos(lifetime = none, skip ".")]
    enum Token<'s> {
        #[token("at", |_| "at")]
        At(&'s str),
    }

    #[test]
    fn test() {
        let source = ('a'..='z')
            .flat_map(|c1| ('a'..='z').map(move |c2| format!("{c1}{c2}")))
            .collect::<String>();
        let mut lexer = <Token<'static> as Logos>::lexer(&source);

        assert_eq!(lexer.next(), Some(Ok(Token::At("at"))));
        assert_eq!(lexer.next(), Some(Ok(Token::At("at"))));
        assert_eq!(lexer.next(), None);
    }
}

mod extras_non_source_lifetime {
    use super::*;

    #[derive(Logos, PartialEq, Debug)]
    #[logos(lifetime = none)]
    #[logos(extras = (usize, &'a [String]))]
    enum Token<'a> {
        #[regex("[0-9]+", |lex| {
            let idx = lex.slice().parse::<usize>().unwrap_or_default();
            lex.extras.0 += idx;
            idx
        })]
        Offset(usize),
        #[token("idx", |lex| {
            let (idx, data) = lex.extras;
            data[idx].as_str()
        })]
        Index(&'a str),
    }

    fn lexer<'s, 'a>(source: &'s str, data: &'a [String]) -> Lexer<'s, Token<'a>> {
        Token::lexer_with_extras(source, (0, data))
    }

    #[test]
    fn test() {
        let source = String::from("266idx81idxidx7idx123idx");
        let data = ('a'..='z')
            .flat_map(|c| (0..99).map(move |i| format!("{c}{i}")))
            .collect::<Vec<_>>();

        let mut lexer = lexer(&source, &data);

        assert_eq!(lexer.next(), Some(Ok(Token::Offset(266))));
        assert_eq!(lexer.next(), Some(Ok(Token::Index(&data[266]))));
        assert_eq!(lexer.next(), Some(Ok(Token::Offset(81))));
        assert_eq!(lexer.next(), Some(Ok(Token::Index(&data[347]))));
        assert_eq!(lexer.next(), Some(Ok(Token::Index(&data[347]))));
        assert_eq!(lexer.next(), Some(Ok(Token::Offset(7))));
        assert_eq!(lexer.next(), Some(Ok(Token::Index(&data[354]))));
        assert_eq!(lexer.next(), Some(Ok(Token::Offset(123))));
        assert_eq!(lexer.next(), Some(Ok(Token::Index(&data[477]))));
        assert_eq!(lexer.next(), None);
    }
}

mod multiple_lifetimes {
    use super::*;

    #[derive(Logos, PartialEq, Debug)]
    #[logos(lifetime = 's)]
    #[logos(skip " +")]
    enum Token<'s, 'a> {
        #[regex(r"[a-z]+", |lex| {
            lex.slice().to_uppercase().leak() as &'static _
        })]
        Capital(&'a str),
        #[regex(r"\([a-z]+\)")]
        Raw(&'s str),
    }

    #[test]
    fn test() {
        let source = String::from("a (bc) def (ghijk) lm");

        let mut lexer = <Token<'_, 'static> as Logos>::lexer(&source);

        assert_eq!(lexer.next(), Some(Ok(Token::Capital("A"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Raw("(bc)"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Capital("DEF"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Raw("(ghijk)"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Capital("LM"))));
        assert_eq!(lexer.next(), None);
    }
}

mod multiple_lifetimes_convoluted {
    use super::*;
    use std::marker::PhantomData;

    #[derive(Logos, PartialEq, Debug)]
    #[logos(lifetime = 's)]
    #[logos(extras = &'e mut dyn FnMut(u32) -> u32)]
    #[logos(skip " +")]
    enum Token<'a, 's, 'e> {
        #[regex(r"[0-9]+", |lex| lex.slice().parse::<u32>().map_or(Filter::Skip, |n| Filter::Emit((lex.extras)(n))))]
        Num(u32),
        #[regex(r"[a-z]+", |lex| {
            lex.slice().to_uppercase().leak() as &'static _
        })]
        Capital(&'a str),
        #[regex(r"\([a-z]+\)")]
        Raw(&'s str),
        _Unreachable(PhantomData<&'e ()>),
    }

    fn lex<'e>(f: &'e mut dyn FnMut(u32) -> u32) -> &'e mut dyn FnMut(u32) -> u32 {
        let source = String::from("6 upper 1 (lower) (a) 0 17 b");
        let mut lexer: Lexer<'_, Token<'static, '_, 'e>> = Token::lexer_with_extras(&source, f);

        assert_eq!(lexer.next(), Some(Ok(Token::Num(6))));
        assert_eq!(lexer.next(), Some(Ok(Token::Capital("UPPER"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Num(61))));
        assert_eq!(lexer.next(), Some(Ok(Token::Raw("(lower)"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Raw("(a)"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Num(610))));
        assert_eq!(lexer.next(), Some(Ok(Token::Num(6117))));
        assert_eq!(lexer.next(), Some(Ok(Token::Capital("B"))));
        assert_eq!(lexer.next(), None);

        lexer.extras
    }

    #[test]
    fn test() {
        let mut total = 0;
        let mut f = |n| {
            total = total * 10 + n;
            total
        };

        assert_eq!(lex(&mut f)(0), 61170);
    }
}

mod lifetime_bounds {
    use super::*;

    #[derive(Logos)]
    #[logos(lifetime = 's)]
    #[logos(extras = &'a str)]
    #[logos(skip ", *")]
    enum Token<'s: 'a, 'a> {
        #[regex("STORE [a-z]+", |lex| {
            lex.extras = lex.slice().trim_start_matches("STORE ");
            lex.slice()
        })]
        Store(&'s str),
        #[token("LOAD", |lex| lex.extras)]
        Load(&'a str),
    }

    #[test]
    fn test() {
        let source = String::from("LOAD, STORE abc, STORE eee, LOAD, LOAD, STORE xyz, LOAD");
        let stored = {
            let extras = String::from("init");

            let mut stored = Vec::new();
            let mut loaded = Vec::new();
            for tok in Token::lexer_with_extras(&source, &extras) {
                match tok {
                    Ok(Token::Load(s)) => loaded.push(s),
                    Ok(Token::Store(s)) => stored.push(s),
                    Err(_) => {}
                }
            }

            assert_eq!(loaded, &["init", "eee", "eee", "xyz"]);

            stored
        };

        assert_eq!(stored, &["STORE abc", "STORE eee", "STORE xyz"]);
    }
}
