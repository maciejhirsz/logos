/// Test against issue #265 on GitHub and duplicates.
use logos_derive::Logos;
use tests::assert_lex;

mod maltejanz {
    /// From https://github.com/maciejhirsz/logos/issues/265
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[allow(non_camel_case_types)]
    pub enum Token {
        #[regex(r"[ \t]+", priority = 1)]
        TK_WHITESPACE = 0,
        #[regex(r"[a-zA-Z][a-zA-Z0-9]*", priority = 1)]
        TK_WORD,
        #[token("not", priority = 50)]
        TK_NOT,
        #[token("not in", priority = 60)]
        TK_NOT_IN,
    }

    #[test]
    fn single_not() {
        assert_lex("not", &[(Ok(Token::TK_NOT), "not", 0..3)]);
    }

    #[test]
    fn word_then_not() {
        assert_lex(
            "word not",
            &[
                (Ok(Token::TK_WORD), "word", 0..4),
                (Ok(Token::TK_WHITESPACE), " ", 4..5),
                (Ok(Token::TK_NOT), "not", 5..8),
            ],
        );
    }

    #[test]
    fn not_then_word() {
        assert_lex(
            "not word",
            &[
                (Ok(Token::TK_NOT), "word", 0..3),
                (Ok(Token::TK_WHITESPACE), " ", 3..4),
                (Ok(Token::TK_WORD), "not", 4..8),
            ],
        );
    }

    #[test]
    fn not_in() {
        assert_lex(
            "not in ",
            &[
                (Ok(Token::TK_NOT_IN), "not in", 0..6),
                (Ok(Token::TK_WHITESPACE), " ", 6..7),
            ],
        );
    }
}

mod jeertmans {
    /// From https://github.com/maciejhirsz/logos/issues/279
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[token(r"\")]
        Backslash,
        #[token(r"\\")]
        DoubleBackslash,
        #[token(r"\begin")]
        EnvironmentBegin,
        #[token(r"\end")]
        EnvironmentEnd,
        #[token(r"\begin{document}")]
        DocumentBegin,
        #[regex(r"\\[a-zA-Z]+")]
        MacroName,
    }

    #[test]
    fn backslash() {
        assert_lex(
            r"\+\\+",
            &[
                (Ok(Token::Backslash), r"\", 0..1),
                (Err(()), "+", 1..2),
                (Ok(Token::DoubleBackslash), r"\\", 2..4),
                (Err(()), "+", 4..5),
            ],
        );
    }

    #[test]
    fn double_backslash() {
        assert_lex(
            r"\\\",
            &[
                (Ok(Token::DoubleBackslash), r"\\", 0..2),
                (Ok(Token::Backslash), r"\", 2..3),
            ],
        );
    }

    #[test]
    fn environment_begin() {
        assert_lex(
            r"\begin{equation}",
            &[(Ok(Token::EnvironmentBegin), r"\begin", 0..6)],
        );
    }

    #[test]
    fn environment_end() {
        assert_lex(
            r"\end{equation}",
            &[(Ok(Token::EnvironmentEnd), r"\end", 0..4)],
        );
    }
}

mod afreeland {
    /// From https://github.com/maciejhirsz/logos/issues/377
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[token("alert")]
        Action,
        #[token("tls")]
        Protocol,
        #[regex(r"([^\s]+) ([^\s]+) (->|<-) ([^\s]+) ([^\s]+)")]
        NetworkInfo,
    }

    #[test]
    fn basic() {
        assert_lex(
            "alert tls $HOME_NET any -> $EXTERNAL_NET any (msg:\"some bs\")",
            &[
                (Ok(Token::Action), "alert", 0..5),
                (Err(()), " ", 5..6),
                (Ok(Token::Action), "tsl", 6..9),
                (Err(()), " ", 9..10),
                (
                    Ok(Token::NetworkInfo),
                    "$HOME_NET any -> $EXTERNAL_NET any (msg:\"some bs\")",
                    10..60,
                ),
            ],
        );
    }
}
