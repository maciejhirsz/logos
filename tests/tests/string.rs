use logos::Lexer;
use logos::Logos;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip " ")]
pub enum Token {
    #[regex(r#""([^"\\]+|\\.)*""#, lex_single_line_string)]
    String(String),
}

#[test]
fn test_it_works_without_cloning() {
    let mut lexer = Token::lexer(r#""Hello, world!" "foo😀bar\nbaz \x3F\u{1234}""#);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::String("Hello, world!".to_string())))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_it_works_with_cloning() {
    let mut lexer = Token::lexer(r#""Hello, world!" "foo😀bar\nbaz \x3F\u{1234}""#);
    let mut lexer2 = lexer.clone();
    assert_eq!(
        lexer2.next(),
        Some(Ok(Token::String("Hello, world!".to_string())))
    );
    let mut lexer3 = lexer2.clone();
    let mut lexer4 = lexer3.clone();
    assert_eq!(
        lexer3.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(
        lexer4.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(lexer4.next(), None);
    let mut lexer5 = lexer.clone();
    assert_eq!(
        lexer5.next(),
        Some(Ok(Token::String("Hello, world!".to_string())))
    );
    assert_eq!(
        lexer5.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::String("Hello, world!".to_string())))
    );
    assert_eq!(lexer5.next(), None);
    assert_eq!(
        lexer2.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(lexer2.next(), None);
    assert_eq!(lexer3.next(), None);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::String("foo😀bar\nbaz \x3F\u{1234}".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

// Not important
pub fn lex_single_line_string(lexer: &mut Lexer<Token>) -> Result<String, ()> {
    let mut string = String::new();
    let mut chars = lexer.slice()[1..lexer.slice().len() - 1].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let c = chars.next().ok_or(())?;
                match c {
                    '\n' => {}
                    'n' => string.push('\n'),
                    'r' => string.push('\r'),
                    't' => string.push('\t'),
                    '0' => string.push('\0'),
                    '\'' | '"' | '\\' => string.push(c),
                    'x' => {
                        let mut hex = String::new();
                        hex.push(chars.next().ok_or(())?);
                        hex.push(chars.next().ok_or(())?);
                        let code = u8::from_str_radix(&hex, 16).map_err(|_| ())?;
                        if code > 0x7F {
                            return Err(());
                        }
                        string.push(code as char);
                    }
                    'u' => {
                        if chars.next() != Some('{') {
                            return Err(());
                        }
                        let mut hex = String::new();
                        for _ in 0..6 {
                            let c = chars.next().ok_or(())?;
                            if c == '}' {
                                break;
                            }
                            hex.push(c);
                        }
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| ())?;
                        string.push(char::from_u32(code).ok_or(())?);
                    }
                    _ => return Err(()),
                }
            }
            _ => string.push(c),
        }
    }
    Ok(string)
}
