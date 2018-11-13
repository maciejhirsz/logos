use syn::Ident;
use std::cmp::Ordering;
use regex::Pattern;
use util::OptionExt;

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub pattern: Pattern,
    pub token: Option<&'a Ident>,
    pub consequents: Vec<Node<'a>>,
    pub fallback: Option<Vec<Pattern>>,
}

impl<'a> Node<'a> {
    pub fn new<P>(pattern: Pattern, path: &mut P, token: &'a Ident) -> Self
    where
        P: Iterator<Item = Pattern>,
    {
        let mut node = Node {
            pattern,
            token: None,
            fallback: None,
            consequents: Vec::new(),
        };

        node.insert(path, token);

        node
    }

    pub fn insert<P>(&mut self, path: &mut P, token: &'a Ident)
    where
        P: Iterator<Item = Pattern>,
    {
        static ERR: &str = "Two patterns resolving to the same token.";

        let pattern = match path.next() {
            Some(pattern) => pattern,
            None => {
                return self.token.insert(token, ERR);
            }
        };

        if let Pattern::Repeat(_) = pattern {
            self.token.insert(token, ERR);
        }

        match self.consequents.binary_search_by(|node| {
            (&node.pattern).partial_cmp(&pattern).unwrap_or_else(|| Ordering::Greater)
        }) {
            Ok(index) => {
                self.consequents[index].insert(path, token);
            },
            Err(index) => {
                self.consequents.insert(index, Node::new(pattern, path, token));
            },
        }
    }

    /// Tests whether the branch produces a token on all leaves without any tests.
    pub fn exhaustive(&self) -> bool {
        self.token.is_some() && self.consequents.iter().all(Self::exhaustive)
    }
}
