use std::ops::{BitAnd, BitOr};

use proc_macro2::{Ident, TokenStream, TokenTree};

use crate::parser::Parser;
use crate::util::is_punct;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IgnoreFlags {
    bits: u8,
}

#[allow(non_upper_case_globals)]
impl IgnoreFlags {
    pub const Empty: Self = Self::new(0x00);
    pub const IgnoreCase: Self = Self::new(0x01);
    pub const IgnoreAsciiCase: Self = Self::new(0x02);

    #[inline]
    pub const fn new(bits: u8) -> Self {
        Self { bits }
    }

    /// Enables a variant.
    #[inline]
    pub fn enable(&mut self, variant: Self) {
        self.bits |= variant.bits;
    }

    /// Checks if this `IgnoreFlags` contains *any* of the given variants.
    #[inline]
    pub fn contains(&self, variants: Self) -> bool {
        self.bits & variants.bits != 0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

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
                if self.contains(Self::IgnoreAsciiCase) {
                    parser.err(
                        "\
                        The flag \"case\" cannot be used along with \"ascii_case\"\
                        ",
                        ident.span(),
                    );
                    false
                } else {
                    self.enable(Self::IgnoreCase);
                    true
                }
            }
            "ascii_case" => {
                if self.contains(Self::IgnoreCase) {
                    parser.err(
                        "\
                        The flag \"ascii_case\" cannot be used along with \"case\"\
                        ",
                        ident.span(),
                    );
                    false
                } else {
                    self.enable(Self::IgnoreAsciiCase);
                    true
                }
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
        // Little finite state machine to parse "<flag>(,<flag>)*,?"

        // FSM description for future maintenance
        // 0: Initial state
        //   <flag> -> 1
        //        _ -> error
        // 1: A flag was found
        //        , -> 2
        //     None -> done
        //        _ -> error
        // 2: A comma was found (after a <flag>)
        //   <flag> -> 1
        //     None -> done
        //        _ -> error
        let mut state = 0u8;

        let mut tokens = tokens.into_iter();

        loop {
            state = match state {
                0 => match tokens.next() {
                    Some(TokenTree::Ident(ident)) => {
                        if self.parse_ident(ident, parser) {
                            1
                        } else {
                            return;
                        }
                    }
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
                },
                1 => match tokens.next() {
                    Some(tt) if is_punct(&tt, ',') => 2,
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
                },
                2 => match tokens.next() {
                    Some(TokenTree::Ident(ident)) => {
                        if self.parse_ident(ident, parser) {
                            1
                        } else {
                            return;
                        }
                    }
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
                },
                _ => unreachable!("Internal Error: invalid state ({})", state),
            }
        }
    }
}

impl BitOr for IgnoreFlags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self::new(self.bits | other.bits)
    }
}

impl BitAnd for IgnoreFlags {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self::new(self.bits & other.bits)
    }
}

