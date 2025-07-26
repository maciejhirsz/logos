use proc_macro2::{Ident, TokenStream, TokenTree};

use crate::parser::Parser;
use crate::util::is_punct;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct IgnoreFlags {
    pub ignore_case: bool,
}

impl IgnoreFlags {
    /// Parses an identifier an enables it for `self`.
    ///
    /// Valid inputs are (that produces `true`):
    /// * `"case"` (incompatible with `"ascii_case"`)
    /// * `"ascii_case"` (incompatible with `"case"`)
    ///
    /// An error causes this function to return `false` and emits an error to
    /// the given `Parser`.
    fn parse_ident(&mut self, ident: Ident, parser: &mut Parser) -> bool {
        match ident.to_string().as_str() {
            "case" => {
                self.ignore_case = true;
                true
            }
            "ascii_case" => {
                parser.err(
                    "\
                    The flag \"ascii_case\" is no longer supported\
                    ",
                    ident.span(),
                );
                false
            }
            unknown => {
                parser.err(
                    format!(
                        "\
                        Unknown flag: {}\n\
                        \n\
                        Expected one of: case, ascii_case\
                        ",
                        unknown
                    ),
                    ident.span(),
                );
                false
            }
        }
    }

    pub fn parse_group(&mut self, name: Ident, tokens: TokenStream, parser: &mut Parser) {
        let mut tokens = tokens.into_iter();
        let mut found_flag = false;

        loop {
            match tokens.next() {
                Some(TokenTree::Ident(ident)) => {
                    if self.parse_ident(ident, parser) {
                        found_flag = true;
                    } else {
                        return;
                    }
                }
                None if found_flag => return,
                _ => {
                    parser.err(
                        "\
                        Invalid ignore flag\n\
                        \n\
                        Expected one of: case, ascii_case\
                        ",
                        name.span(),
                    );
                    return;
                }
            }

            match tokens.next() {
                Some(tt) if is_punct(&tt, ',') => {}
                None => return,
                Some(unexpected_tt) => {
                    parser.err(
                        format!(
                            "\
                            Unexpected token: {:?}\
                            ",
                            unexpected_tt.to_string(),
                        ),
                        unexpected_tt.span(),
                    );
                    return;
                }
            };
        }
    }
}
