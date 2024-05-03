#[derive()]
enum Token {
    Newline,
    AnyUnicode,
    Any,
}
impl<'s> ::logos::Logos<'s> for Token {
    type Error = ();
    type Extras = ();
    type Source = [u8];
    fn lex(lex: &mut ::logos::Lexer<'s, Self>) {
        use logos::internal::{CallbackResult, LexerInternal};
        type Lexer<'s> = ::logos::Lexer<'s, Token>;
        fn _end<'s>(lex: &mut Lexer<'s>) {
            lex.end()
        }
        fn _error<'s>(lex: &mut Lexer<'s>) {
            lex.bump_unchecked(1);
            lex.error();
        }
        macro_rules ! _fast_loop { ($ lex : ident , $ test : ident , $ miss : expr) => { while let Some (arr) = $ lex . read :: < & [u8 ; 16] > () { if $ test (arr [0]) { if $ test (arr [1]) { if $ test (arr [2]) { if $ test (arr [3]) { if $ test (arr [4]) { if $ test (arr [5]) { if $ test (arr [6]) { if $ test (arr [7]) { if $ test (arr [8]) { if $ test (arr [9]) { if $ test (arr [10]) { if $ test (arr [11]) { if $ test (arr [12]) { if $ test (arr [13]) { if $ test (arr [14]) { if $ test (arr [15]) { $ lex . bump_unchecked (16) ; continue ; } $ lex . bump_unchecked (15) ; return $ miss ; } $ lex . bump_unchecked (14) ; return $ miss ; } $ lex . bump_unchecked (13) ; return $ miss ; } $ lex . bump_unchecked (12) ; return $ miss ; } $ lex . bump_unchecked (11) ; return $ miss ; } $ lex . bump_unchecked (10) ; return $ miss ; } $ lex . bump_unchecked (9) ; return $ miss ; } $ lex . bump_unchecked (8) ; return $ miss ; } $ lex . bump_unchecked (7) ; return $ miss ; } $ lex . bump_unchecked (6) ; return $ miss ; } $ lex . bump_unchecked (5) ; return $ miss ; } $ lex . bump_unchecked (4) ; return $ miss ; } $ lex . bump_unchecked (3) ; return $ miss ; } $ lex . bump_unchecked (2) ; return $ miss ; } $ lex . bump_unchecked (1) ; return $ miss ; } return $ miss ; } while $ lex . test ($ test) { $ lex . bump_unchecked (1) ; } $ miss } ; }
        #[inline]
        fn goto1_x<'s>(lex: &mut Lexer<'s>) {
            lex.set(Ok(Token::Newline));
        }
        #[inline]
        fn goto11_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            lex.set(Ok(Token::Any));
        }
        #[inline]
        fn goto2_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            lex.set(Ok(Token::AnyUnicode));
        }
        #[inline]
        fn goto16_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 2usize]>() {
                Some([128u8..=159u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(2usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto17_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 3usize]>() {
                Some([144u8..=191u8, 128u8..=191u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(3usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto2_x<'s>(lex: &mut Lexer<'s>) {
            lex.set(Ok(Token::AnyUnicode));
        }
        #[inline]
        fn goto13_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 1usize]>() {
                Some([128u8..=191u8]) => {
                    lex.bump_unchecked(1usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto18_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 3usize]>() {
                Some([128u8..=191u8, 128u8..=191u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(3usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto15_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 2usize]>() {
                Some([128u8..=191u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(2usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto14_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 2usize]>() {
                Some([160u8..=191u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(2usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto19_ctx11_x<'s>(lex: &mut Lexer<'s>) {
            match lex.read::<&[u8; 3usize]>() {
                Some([128u8..=143u8, 128u8..=191u8, 128u8..=191u8]) => {
                    lex.bump_unchecked(3usize);
                    goto2_ctx11_x(lex)
                }
                _ => goto11_ctx11_x(lex),
            }
        }
        #[inline]
        fn goto11_x<'s>(lex: &mut Lexer<'s>) {
            lex.set(Ok(Token::Any));
        }
        #[inline]
        fn goto20<'s>(lex: &mut Lexer<'s>) {
            enum Jump {
                J1,
                J16,
                J17,
                J2,
                J13,
                J18,
                J15,
                J14,
                J19,
                J11,
            }
            const LUT: [Jump; 256] = {
                use Jump::*;
                [
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J1, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J11, J11, J11, J11, J11, J11, J11, J11, J11,
                    J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11,
                    J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11,
                    J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11, J11,
                    J11, J11, J11, J11, J11, J11, J11, J11, J11, J13, J13, J13, J13, J13, J13, J13,
                    J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13, J13,
                    J13, J13, J13, J13, J13, J13, J13, J14, J15, J15, J15, J15, J15, J15, J15, J15,
                    J15, J15, J15, J15, J16, J15, J15, J17, J18, J18, J18, J19, J11, J11, J11, J11,
                    J11, J11, J11, J11, J11, J11, J11,
                ]
            };
            let byte = match lex.read::<u8>() {
                Some(byte) => byte,
                None => return _end(lex),
            };
            match LUT[byte as usize] {
                Jump::J1 => {
                    lex.bump_unchecked(1usize);
                    goto1_x(lex)
                }
                Jump::J16 => {
                    lex.bump_unchecked(1usize);
                    goto16_ctx11_x(lex)
                }
                Jump::J17 => {
                    lex.bump_unchecked(1usize);
                    goto17_ctx11_x(lex)
                }
                Jump::J2 => {
                    lex.bump_unchecked(1usize);
                    goto2_x(lex)
                }
                Jump::J13 => {
                    lex.bump_unchecked(1usize);
                    goto13_ctx11_x(lex)
                }
                Jump::J18 => {
                    lex.bump_unchecked(1usize);
                    goto18_ctx11_x(lex)
                }
                Jump::J15 => {
                    lex.bump_unchecked(1usize);
                    goto15_ctx11_x(lex)
                }
                Jump::J14 => {
                    lex.bump_unchecked(1usize);
                    goto14_ctx11_x(lex)
                }
                Jump::J19 => {
                    lex.bump_unchecked(1usize);
                    goto19_ctx11_x(lex)
                }
                Jump::J11 => {
                    lex.bump_unchecked(1usize);
                    goto11_x(lex)
                }
            }
        }
        goto20(lex)
    }
}
