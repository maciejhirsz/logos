use std::cell::Cell;

use logos::Logos;

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token {
    #[token("a", |_| Evil::default())]
    Evil(Evil),
}

#[derive(Debug, Default, PartialEq)]
pub struct Evil(Box<Cell<u8>>);

impl Clone for Evil {
    fn clone(&self) -> Self {
        self.0.set(self.0.get() + 1);
        Self::default()
    }
}

#[test]
fn test_it_works_without_cloning() {
    let mut lexer = Token::lexer("aaa");
    assert_eq!(lexer.next(), Some(Ok(Token::Evil(Evil::default()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Evil(Evil::default()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Evil(Evil::default()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_clone_ub() {
    let mut lexer = Token::lexer("a");
    assert_eq!(lexer.next(), Some(Ok(Token::Evil(Evil::default()))));

    // In logos 0.14.1, this causes use-after-free (UB),
    // because `Clone` dereferences the value returned by the last call to `lexer.next()`,
    // which got deallocated.
    // A real-life example where this could happen is with `Rc`.
    // Note that it may still pass `cargo test`, it will fail with Miri.
    let mut lexer2 = lexer.clone();

    assert_eq!(lexer2.next(), None);
}

#[test]
fn test_clone_leak() {
    let mut lexer = Token::lexer("a");
    let Some(Ok(Token::Evil(evil))) = lexer.next() else {
        panic!("Expected Token::Evil");
    };
    assert_eq!(evil.0.get(), 0);

    // In logos 0.14.1, this causes a memory leak because `evil` is cloned with `lexer`.
    // This produces `evil.0.get() == 1`. It will fail even on `cargo test`.
    let mut lexer2 = lexer.clone();
    assert_eq!(evil.0.get(), 0);

    assert_eq!(lexer2.next(), None);
    let _ = evil.clone();
    assert_eq!(evil.0.get(), 1);
}
