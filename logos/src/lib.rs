pub type Lexicon<Logos, Source> = [Option<fn(&mut Lexer<Logos, Source>)>; 256];

/// Trait that will be derived for the appropriate enum representing all the tokens
pub trait Logos: Sized {
    /// Associated `Extras` for the particular lexer. Those can handle things that
    /// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
    /// in JavaScript.
    type Extras: self::Extras;

    /// `SIZE` is simply a number of possible variants of the `Logos` enum. The
    /// `derive` macro will make sure that all variants don't hold values larger
    /// or equal to `SIZE`.
    ///
    /// This can be extremely useful for creating `Logos` Lookup Tables.
    const SIZE: usize;

    /// Helper const pointing to the error `Logos` variant.
    const ERROR: Self;

    fn lexicon<S: Source>() -> Lexicon<Self, S>;
}

pub trait Extras: Sized + Default {
    fn on_consume(&mut self);
    fn on_whitespace(&mut self, byte: u8);
}

impl Extras for () {
    fn on_consume(&mut self) {}
    fn on_whitespace(&mut self, _byte: u8) {}
}

pub trait Source {
    type Slice;

    fn read(&self, offset: usize) -> u8;
    fn slice(&self, from: usize, to: usize) -> Self::Slice;
}

impl<'source> Source for &'source str {
    type Slice = &'source str;

    fn read(&self, offset: usize) -> u8 {
        self.as_bytes()
            .get(offset)
            .map(Clone::clone)
            .unwrap_or_else(|| 0)
    }

    fn slice(&self, from: usize, to: usize) -> Self::Slice {
        &self[from..to]
    }
}

pub struct Lexer<Token: Logos, Source> {
    source: Source,
    token_start: usize,
    token_end: usize,
    pub token: Token,
    pub extras: Token::Extras,
    lexicon: [Option<fn(&mut Lexer<Token, Source>)>; 256],
}

impl<Token: Logos, S: Source> Lexer<Token, S> {
    pub fn new(source: S) -> Self {
        let mut lex = Lexer {
            source,
            token_start: 0,
            token_end: 0,
            token: Token::ERROR,
            extras: Default::default(),
            lexicon: Token::lexicon(),
        };

        lex.consume();

        lex
    }

    pub fn loc(&self) -> (usize, usize) {
        (self.token_start, self.token_end)
    }

    pub fn consume(&mut self) {
        let mut ch;

        self.extras.on_consume();

        loop {
            ch = self.read();

            if let Some(handler) = self.lexicon[ch as usize] {
                self.token_start = self.token_end;
                return handler(self);
            }

            self.extras.on_whitespace(ch);

            self.bump();
        }
    }

    pub fn read(&mut self) -> u8 {
        self.source.read(self.token_end)
    }

    pub fn next(&mut self) -> u8 {
        self.bump();
        self.read()
    }

    pub fn bump(&mut self) {
        self.token_end += 1;
    }

    pub fn slice(&self) -> S::Slice {
        self.source.slice(self.token_start, self.token_end)
    }
}
