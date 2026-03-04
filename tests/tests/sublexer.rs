use logos::Lexer;
use logos::Logos;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
#[logos(extras = &'e mut bool)]
enum Alpha<'s> {
    #[regex("[a-z]+")]
    Value(&'s str),
    #[token("|")]
    Swap,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
enum Numeric<'s> {
    #[regex("[0-9]+")]
    Value(&'s str),
    #[token("|")]
    Swap,
}

#[derive(Debug, PartialEq, Eq)]
enum AlphaNumeric<'s> {
    Alpha(&'s str),
    Numeric(&'s str),
    Swap,
}

impl<'s> Logos<'s> for AlphaNumeric<'s> {
    type Extras<'e> = bool;
    type Source = str;
    type Error = ();
    fn lex<'b>(lexer: &mut Lexer<'s, '_, Self>) -> Option<Result<Self, Self::Error>> {
        if !lexer.extras {
            let result = {
                let mut sublexer = lexer.sublexer_with::<Alpha>(|that| that);
                sublexer.next()?
            };
            let Ok(Alpha::Value(that)) = result else {
                lexer.extras = !lexer.extras;
                return Some(Ok(AlphaNumeric::Swap));
            };
            return Some(Ok(AlphaNumeric::Alpha(that)));
        } else {
            let result = {
                let mut sublexer = lexer.sublexer::<Numeric>();
                sublexer.next()?
            };
            let Ok(Numeric::Value(that)) = result else {
                lexer.extras = !lexer.extras;
                return Some(Ok(AlphaNumeric::Swap));
            };
            return Some(Ok(AlphaNumeric::Numeric(that)));
        }
    }
}

#[test]
fn sublexer_with_modes() {
    let s = r#"abc|123|def"#;

    let results: Vec<Result<AlphaNumeric, ()>> = AlphaNumeric::lexer(s).collect();
    let expect = vec![
        Ok(AlphaNumeric::Alpha("abc")),
        Ok(AlphaNumeric::Swap),
        Ok(AlphaNumeric::Numeric("123")),
        Ok(AlphaNumeric::Swap),
        Ok(AlphaNumeric::Alpha("def")),
    ];
    assert_eq!(results, expect);
}
