// test case that return faulty Tokens for partial match

use logos_derive::Logos;
use tests::assert_lex;
// use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    // Match anything until the start of PHP code
    #[regex(r#"[^<]+"#, |lex| lex.slice().to_string())]
    Text(String),

    // Match PHP open tag
    #[token("<?php")]
    PhpStartLong,

    #[token("<?")]
    PhpStartShort,

    // match php echo statement
    #[token("<?=")]
    PhpEcho,

    #[token("<")]
    StartTag,
}

#[test]
fn only_valid_tags() {
    assert_lex(
        "123<<?<?=<?php",
        &[
            (Ok(Token::Text("123".to_string())), "123", 0..3),
            (Ok(Token::StartTag), "<", 3..4),
            (Ok(Token::PhpStartShort), "<?", 4..6),
            (Ok(Token::PhpEcho), "<?=", 6..9),
            (Ok(Token::PhpStartLong), "<?php", 9..14),
        ],
    );
}

#[test]
fn failing_tags_tokens() {
    assert_lex(
        "<?a <?p <?ph <?php <?=a",
        &[
            (Ok(Token::PhpStartShort), "<?", 0..2),
            (Ok(Token::Text("a ".to_string())), "a ", 2..4),
            (Ok(Token::PhpStartShort), "<?", 4..6),
            (Ok(Token::Text("p ".to_string())), "p ", 6..8),
            (Ok(Token::PhpStartShort), "<?", 8..10),
            (Ok(Token::Text("ph ".to_string())), "ph ", 10..13),
            (Ok(Token::PhpStartLong), "<?php", 13..18),
            (Ok(Token::Text(" ".to_string())), " ", 18..19),
            (Ok(Token::PhpEcho), "<?=", 19..22),
            (Ok(Token::Text("a".to_string())), "a", 22..23),
        ],
    );
}
