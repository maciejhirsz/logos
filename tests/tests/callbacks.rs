use logos::{Lexer, Logos as _, Skip};
use logos_derive::Logos;
use tests::assert_lex;

#[derive(Default, Debug, Clone, PartialEq)]
enum LexingError {
    ParseNumberError,
    #[default]
    Other,
}

impl From<std::num::ParseIntError> for LexingError {
    fn from(_: std::num::ParseIntError) -> Self {
        LexingError::ParseNumberError
    }
}

impl From<std::num::ParseFloatError> for LexingError {
    fn from(_: std::num::ParseFloatError) -> Self {
        LexingError::ParseNumberError
    }
}

mod data {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(error = LexingError)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token<'a> {
        #[regex(r"[a-zA-Z]+", |lex| lex.slice())]
        Text(&'a str),

        #[regex(r"-?[0-9]+", |lex| lex.slice().parse())]
        Integer(i64),

        #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse())]
        Float(f64),
    }

    #[test]
    fn numbers() {
        let tokens: Vec<_> = Token::lexer("Hello 1 42 -100 pi 3.14 -77.77").collect();

        assert_eq!(
            tokens,
            &[
                Ok(Token::Text("Hello")),
                Ok(Token::Integer(1)),
                Ok(Token::Integer(42)),
                Ok(Token::Integer(-100)),
                Ok(Token::Text("pi")),
                Ok(Token::Float(3.14)),
                Ok(Token::Float(-77.77)),
            ]
        );
    }
}

mod nested_lifetime {
    use super::*;
    use std::borrow::Cow;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(error = LexingError)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token<'a> {
        #[regex(r"[0-9]+", |lex| {
            let slice = lex.slice();

            slice.parse::<u64>().map(|n| {
                (slice, n)
            })
        })]
        Integer((&'a str, u64)),

        #[regex(r"[a-z]+", |lex| Cow::Borrowed(lex.slice()))]
        Text(Cow<'a, str>),
    }

    #[test]
    fn supplement_lifetime_in_types() {
        let tokens: Vec<_> = Token::lexer("123 hello 42").collect();

        assert_eq!(
            tokens,
            &[
                Ok(Token::Integer(("123", 123))),
                Ok(Token::Text(Cow::Borrowed("hello"))),
                Ok(Token::Integer(("42", 42))),
            ],
        );
    }
}

mod rust {
    use super::*;

    /// Adaptation of implementation by matklad:
    /// https://github.com/matklad/fall/blob/527ab331f82b8394949041bab668742868c0c282/lang/rust/syntax/src/rust.fall#L1294-L1324
    fn parse_raw_string(lexer: &mut Lexer<Token>) -> bool {
        // Who needs more then 25 hashes anyway? :)
        let q_hashes = concat!('"', "######", "######", "######", "######", "######");
        let closing = &q_hashes[..lexer.slice().len() - 1]; // skip initial 'r'

        lexer
            .remainder()
            .find(closing)
            .map(|i| lexer.bump(i + closing.len()))
            .is_some()
    }

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(error = LexingError)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token {
        #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
        Ident,

        #[regex("r#*\"", parse_raw_string)]
        RawString,
    }

    #[test]
    fn raw_strings() {
        assert_lex(
            " r\"foo\" r#\"bar\"# r#####\"baz\"##### r###\"error\"## ",
            &[
                (Ok(Token::RawString), "r\"foo\"", 1..7),
                (Ok(Token::RawString), "r#\"bar\"#", 8..16),
                (Ok(Token::RawString), "r#####\"baz\"#####", 17..33),
                (Err(LexingError::Other), "r###\"", 34..39),
                (Ok(Token::Ident), "error", 39..44),
                (Err(LexingError::Other), "\"", 44..45),
                (Err(LexingError::Other), "#", 45..46),
                (Err(LexingError::Other), "#", 46..47),
            ],
        );
    }
}

mod any_token_callback {
    use super::*;

    // Adaption of data test for (_) -> Token callbacks
    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token {
        #[regex(r"[a-zA-Z]+", |_| Token::Text)]
        #[regex(r"-?[0-9]+", |_| Token::Integer)]
        #[regex(r"-?[0-9]+\.[0-9]+", |_| Token::Float)]
        Text,
        Integer,
        Float,
    }

    #[test]
    fn any_token_callback() {
        let tokens: Vec<_> = Token::lexer("Hello 1 42 -100 pi 3.14 -77.77").collect();

        assert_eq!(
            tokens,
            &[
                Ok(Token::Text),
                Ok(Token::Integer),
                Ok(Token::Integer),
                Ok(Token::Integer),
                Ok(Token::Text),
                Ok(Token::Float),
                Ok(Token::Float),
            ]
        );
    }
}

mod return_result_skip {
    use super::*;

    #[derive(Debug, Default, PartialEq, Clone)]
    enum LexerError {
        UnterminatedComment,
        #[default]
        Other,
    }

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    #[logos(error = LexerError)]
    enum Token<'src> {
        #[regex(r"<[a-zA-Z0-9-]+>", |lex| &lex.slice()[1..lex.slice().len()-1])]
        Tag(&'src str),

        #[token("<!--", skip_comment)]
        Comment,
    }

    fn skip_comment<'src>(lexer: &mut Lexer<'src, Token<'src>>) -> Result<Skip, LexerError> {
        let end = lexer
            .remainder()
            .find("-->")
            .ok_or(LexerError::UnterminatedComment)?;
        lexer.bump(end + 3);

        Ok(Skip)
    }

    #[test]
    fn return_result_skip() {
        let mut lexer = Token::lexer("<foo> <!-- comment --> <bar>");
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("foo"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("bar"))));
        assert_eq!(lexer.next(), None);

        let mut lexer = Token::lexer("<foo> <!-- unterminated comment");
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("foo"))));
        assert_eq!(lexer.next(), Some(Err(LexerError::UnterminatedComment)));
    }
}

mod skip_callback_function {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    #[logos(skip("<!--", skip_comment))]
    enum Token<'src> {
        #[regex(r"<[a-zA-Z0-9-]+>", |lex| &lex.slice()[1..lex.slice().len()-1])]
        Tag(&'src str),
    }

    fn skip_comment<'src>(lexer: &mut Lexer<'src, Token<'src>>) {
        let end = lexer
            .remainder()
            .find("-->")
            .map(|id| id + 3)
            .unwrap_or(lexer.remainder().len());
        lexer.bump(end);
    }

    #[test]
    fn skip_callback_function() {
        let mut lexer = Token::lexer("<foo> <!-- comment --> <bar>");
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("foo"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("bar"))));
        assert_eq!(lexer.next(), None);

        let mut lexer = Token::lexer("<foo> <!-- unterminated comment");
        assert_eq!(lexer.next(), Some(Ok(Token::Tag("foo"))));
        assert_eq!(lexer.next(), None);
    }
}

#[cfg(test)]
mod skip_callback_closure {
    use super::*;

    #[derive(Debug, Clone, Copy, Default)]
    struct Extras {
        line_num: usize,
    }

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \r]")]
    #[logos(skip(r"\n", callback = |lex| { lex.extras.line_num += 1; }))]
    #[logos(extras = Extras)]
    enum Token {
        #[regex("[a-z]+")]
        Letters,
        #[regex("[0-9]+")]
        Numbers,
    }

    #[test]
    fn skip_callback_closure() {
        let mut lexer = Token::lexer(concat!("abc 123\n", "ab( |23\n", "Abc 123\n",));
        let mut tokens = Vec::new();
        let mut error_lines: Vec<usize> = Vec::new();

        while let Some(token_result) = lexer.next() {
            if let Ok(token) = token_result {
                tokens.push(token);
            } else {
                error_lines.push(lexer.extras.line_num);
            }
        }

        assert_eq!(
            tokens.as_slice(),
            &[
                Token::Letters,
                Token::Numbers,
                Token::Letters,
                Token::Numbers,
                Token::Letters,
                Token::Numbers,
            ]
        );
        assert_eq!(error_lines.as_slice(), &[1, 1, 2]);
    }
}

#[cfg(test)]
mod skip_equal_priorities {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    // Ignore all other characters (. doesn't match newlines)
    #[logos(skip("(.|\n)", priority = 1))]
    enum Token<'s> {
        #[regex(r"[0-9]+")]
        Numbers(&'s str),
        #[regex(r"[!@#$%^&*]")]
        Op(&'s str),
    }

    #[test]
    fn skip_equal_priorities() {
        let tokens: Result<Vec<Token>, _> = Lexer::new(
            "123all letters are considered comments456
            *other characters too& % 789😊! @ 012anything not a number or an op345 678 ^ 901",
        )
        .collect();

        assert_eq!(
            tokens.as_ref().map(Vec::as_slice),
            Ok([
                Token::Numbers("123"),
                Token::Numbers("456"),
                Token::Op("*"),
                Token::Op("&"),
                Token::Op("%"),
                Token::Numbers("789"),
                Token::Op("!"),
                Token::Op("@"),
                Token::Numbers("012"),
                Token::Numbers("345"),
                Token::Numbers("678"),
                Token::Op("^"),
                Token::Numbers("901"),
            ]
            .as_slice())
        );
    }
}
