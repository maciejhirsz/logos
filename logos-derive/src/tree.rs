mod node;
mod leaf;
mod fork;
mod branch;

pub use self::fork::ForkKind::*;
pub use self::fork::{Fork, ForkKind};
pub use self::leaf::{Leaf, Token};
pub use self::node::Node;
pub use self::branch::Branch;

#[cfg(test)]
mod tests {
    use crate::regex::Pattern;
    use super::*;
    use syn::Ident;

    fn token(mock: &str) -> Ident {
        use ::proc_macro2::Span;

        Ident::new(mock, Span::call_site())
    }

    #[test]
    fn branch_to_node() {
        let branch = Branch::new("abc");

        assert_eq!(branch.to_node(), Some(Node::Branch(Branch::new("abc"))));
    }

    #[test]
    fn empty_branch_to_node() {
        let branch = Branch::default();

        assert_eq!(branch.to_node(), None);
    }

    #[test]
    fn empty_branch_with_then_to_node() {
        let token = token("mock");

        let branch = Branch::default().then(&token);

        assert_eq!(branch.to_node(), Some(Node::Leaf(Leaf::from(&token))));
    }

    #[test]
    fn insert_branch_into_branch() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Branch::new("abc").then(&token_a).to_node().unwrap();

        parent.insert(Branch::new("def").then(&token_b));

        assert_eq!(parent, Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(&token_b))
        ));
    }

    #[test]
    fn insert_a_token_into_a_branch() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Branch::new("abc").then(&token_a).to_node().unwrap();

        parent.insert(Leaf::from(&token_b));

        assert_eq!(parent, Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .then(&token_b)
        ));
    }

    #[test]
    fn insert_a_branch_into_a_token() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Leaf(Leaf::from(&token_a));

        parent.insert(Branch::new("xyz").then(&token_b));

        assert_eq!(parent, Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("xyz").then(&token_b))
                .then(&token_a)
        ));
    }

    #[test]
    fn insert_a_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
        );

        parent.insert(
            Fork::new(Plain)
                .arm(Branch::new("def").then(&token_b))
        );

        assert_eq!(parent, Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(&token_b))
        ));
    }

    #[test]
    fn insert_a_maybe_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");
        let token_x = token("XYZ");

        let mut parent = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
        );

        parent.insert(
            Fork::new(Maybe)
                .arm(Branch::new("def").then(&token_b))
                .then(&token_x)
        );

        assert_eq!(parent, Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(&token_b))
                .then(&token_x)
        ));
    }

    #[test]
    fn insert_a_fork_into_a_maybe_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");
        let token_x = token("XYZ");

        let mut parent = Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .then(&token_x)
        );

        parent.insert(
            Fork::new(Plain)
                .arm(Branch::new("def").then(&token_b))
        );

        assert_eq!(parent, Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(&token_b))
                .then(&token_x)
        ));
    }

    #[test]
    fn collapsing_a_fork() {
        let token_a = token("ABC");

        let mut fork =
            Fork::new(Maybe)
                .arm(Branch::new("abc"))
                .then(&token_a);

        let expected =
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .then(&token_a);

        fork.collapse();

        assert_eq!(fork, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn unwinding_a_fork() {
        let token_a = token("ABC");

        let mut fork =
            Fork::new(Repeat)
                .arm(Branch::new("abc"))
                .then(&token_a);

        let expected =
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(fork.clone()))
                .then(&token_a);

        fork.unwind();

        assert_eq!(fork, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn insert_a_repeat_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
        );

        parent.insert(
            Fork::new(Repeat)
                .arm(Branch::new("def"))
                .then(&token_b)
        );

        let expected = Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(
                    Fork::new(Repeat)
                        .arm(Branch::new("def"))
                        .then(&token_b)
                ))
                .then(&token_b)
        );

        assert_eq!(parent, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", parent, expected);
    }

    #[test]
    fn insert_a_fork_into_a_repeat_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(
            Fork::new(Repeat)
                .arm(Branch::new("def"))
                .then(&token_b)
        );

        parent.insert(
            Fork::new(Plain)
                .arm(Branch::new("abc").then(&token_a))
        );

        let expected = Node::Fork(
            Fork::new(Maybe)
                .arm(Branch::new("abc").then(&token_a))
                .arm(Branch::new("def").then(
                    Fork::new(Repeat)
                        .arm(Branch::new("def"))
                        .then(&token_b)
                ))
                .then(&token_b)
        );

        assert_eq!(parent, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", parent, expected);
    }

    #[test]
    fn pack_analog_branches() {
        let mut node = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("a"))
                .arm(Branch::new("b"))
                .arm(Branch::new("c"))
        );

        node.pack();

        let expected = Node::Branch(
            Branch::new(Pattern::Range(b'a', b'c'))
        );

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn pack_analog_branches_with_leaves() {
        let token = token("ABC");

        let mut node = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("a").then(&token))
                .arm(Branch::new("b").then(&token))
                .arm(Branch::new("c").then(&token))
        );

        node.pack();

        let expected = Node::Branch(
            Branch::new(Pattern::Range(b'a', b'c')).then(&token)
        );

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn pack_analog_branches_with_some_leaves() {
        let token = token("ABC");

        let mut node = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("a").then(&token))
                .arm(Branch::new("b"))
                .arm(Branch::new("c").then(&token))
                .arm(Branch::new("d"))
                .arm(Branch::new("e").then(&token))
                .arm(Branch::new("f"))
        );

        node.pack();

        let expected = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new(Pattern::from(&b"ace"[..])).then(&token))
                .arm(Branch::new(Pattern::from(&b"bdf"[..])))
        );

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn pack_analog_branches_with_different_leaves() {
        let abc = token("ABC");
        let def = token("DEF");

        let mut node = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new("a").then(&abc))
                .arm(Branch::new("b").then(&abc))
                .arm(Branch::new("c").then(&abc))
                .arm(Branch::new("d").then(&def))
                .arm(Branch::new("e").then(&def))
                .arm(Branch::new("f").then(&def))
        );

        node.pack();

        let expected = Node::Fork(
            Fork::new(Plain)
                .arm(Branch::new(Pattern::Range(b'a', b'c')).then(&abc))
                .arm(Branch::new(Pattern::Range(b'd', b'f')).then(&def))
        );

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn pack_branches() {
        let mut node = Node::Branch(
            Branch::new("abc").then(
                Branch::new("def").then(
                    Branch::new("xyz")
                )
            )
        );

        node.pack();

        let expected = Node::Branch(
            Branch::new("abcdefxyz")
        );

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn pack_branches_with_a_fork() {
        let mut node = Node::from_regex("abc(1|2|3|4|5)xyz", None);

        assert_eq!(node, Node::Branch(
            Branch::new("abc").then(
                Fork::new(Plain)
                    .arm(Branch::new("1").then(Branch::new("xyz")))
                    .arm(Branch::new("2").then(Branch::new("xyz")))
                    .arm(Branch::new("3").then(Branch::new("xyz")))
                    .arm(Branch::new("4").then(Branch::new("xyz")))
                    .arm(Branch::new("5").then(Branch::new("xyz")))
            )
        ));

        node.pack();

        let expected = Node::Branch(Branch::new(&[
            Pattern::Byte(b'a'),
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
            Pattern::Range(b'1', b'5'),
            Pattern::Byte(b'x'),
            Pattern::Byte(b'y'),
            Pattern::Byte(b'z'),
        ][..]));

        assert_eq!(node, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", node, expected);
    }

    #[test]
    fn fork_insert() {
        let int = token("INTEGER");
        let hex = token("HEX");

        let int_node = Node::from_regex("[0-9]+", Some(Leaf::from(&int)));
        let hex_node = Node::from_regex("0x[0-9a-f]+", Some(Leaf::from(&hex)));

        let mut fork = Fork::new(Plain);

        fork.insert(int_node);
        fork.insert(hex_node);
        fork.pack();

        let expected =
            Fork::new(Plain)
                .arm(
                    Branch::new("0").then(
                        Fork::new(Maybe)
                            .arm(
                                Branch::new(&[
                                    Pattern::Byte(b'x'),
                                    Pattern::from(&b"0123456789abcdef"[..]),
                                ][..]).then(
                                    Fork::new(Repeat)
                                        .arm(Branch::new(Pattern::from(&b"0123456789abcdef"[..])))
                                        .then(&hex)
                                )
                            )
                            .arm(
                                Branch::new(Pattern::Range(b'0', b'9'))
                                    .then(
                                        Fork::new(Repeat)
                                            .arm(Branch::new(Pattern::Range(b'0', b'9')))
                                            .then(&int)
                                    )
                            )
                            .then(&int)
                    )
                )
                .arm(
                    Branch::new(Pattern::Range(b'1', b'9')).then(
                        Fork::new(Repeat)
                            .arm(Branch::new(Pattern::Range(b'0', b'9')))
                            .then(&int)
                    )
                );

        let mut packed = expected.clone();

        packed.pack();

        assert_eq!(packed, expected, "Not equal:\n\nPACKED {:#?}\n\nEXPECTED {:#?}", packed, expected);
        assert_eq!(fork, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn fork_insert_fallback() {
        let ab = token("AB");
        let a  = token("A");

        let ab_node = Node::from_regex("[ab]+", Some(Leaf::from(&ab)));
        let ac_node = Node::from_regex("a+", Some(Leaf::from(&a)));

        let mut fork = Fork::new(Plain);

        fork.insert(ab_node);
        fork.insert(ac_node);
        fork.pack();

        let expected =
            Fork::new(Plain)
                .arm(
                    Branch::new("a")
                        .then(Fork::new(Repeat).arm(Branch::new("a")).then(&a))
                        .fallback(Fork::new(Repeat).arm(Branch::new(Pattern::from(&b"ab"[..]))).then(&ab))
                )
                .arm(
                    Branch::new("b")
                        .then(Fork::new(Repeat).arm(Branch::new(Pattern::from(&b"ab"[..]))).then(&ab))
                );

        assert_eq!(fork, expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn fork_insert_ordering() {
        let int   = token("INTEGER");
        let hex   = token("HEX");
        let float = token("FLOAT");

        let int_node = Node::from_regex("[0-9]+", Some(Leaf::from(&int)));
        let hex_node = Node::from_regex("0x[0-9a-f]+", Some(Leaf::from(&hex)));
        let float_node = Node::from_regex("[0-9]+\\.[0-9]+", Some(Leaf::from(&float)));

        let mut fork_a = Fork::new(Plain);
        let mut fork_b = Fork::new(Plain);
        let mut fork_c = Fork::new(Plain);

        fork_a.insert(int_node.clone());
        fork_a.insert(hex_node.clone());
        fork_a.insert(float_node.clone());
        fork_a.pack();

        fork_b.insert(float_node.clone());
        fork_b.insert(hex_node.clone());
        fork_b.insert(int_node.clone());
        fork_b.pack();

        fork_c.insert(hex_node.clone());
        fork_c.insert(int_node.clone());
        fork_c.insert(float_node.clone());
        fork_c.pack();

        assert_eq!(fork_a, fork_b, "Not equal:\n\nA {:#?}\n\nB {:#?}", fork_a, fork_b);
        assert_eq!(fork_a, fork_c, "Not equal:\n\nA {:#?}\n\nC {:#?}", fork_a, fork_c);
    }
}
