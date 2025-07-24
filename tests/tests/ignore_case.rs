mod ignore_ascii_case {
    use logos_derive::Logos;
    use tests::assert_lex;

    #[derive(Logos, Debug, PartialEq, Eq)]
    #[logos(skip " +")]
    #[logos(utf8 = false)]
    enum Words {
        #[token(b"lOwERCaSe", ignore(case))]
        Lowercase,
        #[token(b"or", ignore(case))]
        Or,
        #[token(b"UppeRcaSE", ignore(case))]
        Uppercase,
        #[token(b":", ignore(case))]
        Colon,
        #[token(b"ThAT", ignore(case))]
        That,
        #[token(b"IS", ignore(case))]
        Is,
        #[token(b"the", ignore(case))]
        The,
        #[token(b"QuEsTiOn", ignore(case))]
        Question,

        #[token(b"MON", ignore(case))]
        Mon,
        #[token(b"fr\xE8RE", ignore(case))]
        Frere,
        #[token(b"\xC9TAIT", ignore(case))]
        Etait,
        #[token(b"l\xE0", ignore(case))]
        La,
        #[token(b"cET", ignore(case))]
        Cet,
        #[token(b"\xE9T\xE9", ignore(case))]
        Ete,
    }

    // #[test]
    // fn tokens_simple() {
    //     assert_lex(
    //         b"LowErcase or UppeRCase: ThAT iS tHe question" as &[u8],
    //         &[
    //             (Ok(Words::Lowercase), b"LowErcase", 0..9),
    //             (Ok(Words::Or), b"or", 10..12),
    //             (Ok(Words::Uppercase), b"UppeRCase", 13..22),
    //             (Ok(Words::Colon), b":", 22..23),
    //             (Ok(Words::That), b"ThAT", 24..28),
    //             (Ok(Words::Is), b"iS", 29..31),
    //             (Ok(Words::The), b"tHe", 32..35),
    //             (Ok(Words::Question), b"question", 36..44),
    //         ],
    //     )
    // }

    #[test]
    fn tokens_nonascii() {
        assert_lex(
            "Mon Frère Était lÀ cet Été".as_bytes(),
            &[
                (Ok(Words::Mon), "Mon".as_bytes(), 0..3),
                (Ok(Words::Frere), "Frère".as_bytes(), 4..10),
                (Ok(Words::Etait), "Était".as_bytes(), 11..17),
                (Err(()), "l".as_bytes(), 18..19),
                (Err(()), "À".as_bytes(), 19..21),
                (Ok(Words::Cet), "cet".as_bytes(), 22..25),
                (Err(()), "É".as_bytes(), 26..28),
                (Err(()), "t".as_bytes(), 28..29),
                (Err(()), "é".as_bytes(), 29..31),
            ],
        )
    }

    // #[derive(Logos, Debug, PartialEq, Eq)]
    // #[logos(skip " +")]
    // #[logos(utf8 = false)]
    // enum Letters {
    //     #[regex("a", ignore(case))]
    //     Single,
    //     #[regex("bc", ignore(case))]
    //     Concat,
    //     #[regex("[de]", ignore(case))]
    //     Altern,
    //     #[regex("f+", ignore(case))]
    //     Loop,
    //     #[regex("gg?", ignore(case))]
    //     Maybe,
    //     #[regex("[h-k]+", ignore(case))]
    //     Range,
    //
    //     #[regex("à", ignore(case))]
    //     NaSingle,
    //     #[regex("éèd", ignore(case))]
    //     NaConcat,
    //     #[regex("[cûü]+", ignore(case))]
    //     NaAltern,
    //     #[regex("i§?", priority = 3, ignore(case))]
    //     NaMaybe,
    //     #[regex("[x-à]+", ignore(case))]
    //     NaRange,
    // }
    //
    // #[test]
    // fn regex_simple() {
    //     assert_lex(
    //         b"aA BCbC DdEE fFff g gg hHiIjJkK" as &[u8],
    //         &[
    //             (Ok(Letters::Single), b"a", 0..1),
    //             (Ok(Letters::Single), b"A", 1..2),
    //             (Ok(Letters::Concat), b"BC", 3..5),
    //             (Ok(Letters::Concat), b"bC", 5..7),
    //             (Ok(Letters::Altern), b"D", 8..9),
    //             (Ok(Letters::Altern), b"d", 9..10),
    //             (Ok(Letters::Altern), b"E", 10..11),
    //             (Ok(Letters::Altern), b"E", 11..12),
    //             (Ok(Letters::Loop), b"fFff", 13..17),
    //             (Ok(Letters::Maybe), b"g", 18..19),
    //             (Ok(Letters::Maybe), b"gg", 20..22),
    //             (Ok(Letters::Range), b"hHiIjJkK", 23..31),
    //         ],
    //     )
    // }
    //
    // #[test]
    // fn regex_nonascii() {
    //     assert_lex(
    //         b"à À éèD Éèd CcûÛüÜC i i§ xXyYzZ|{}" as &[u8],
    //         &[
    //             (Ok(Letters::NaSingle), b"à", 0..2),
    //             (Ok(Letters::NaRange), b"À", 3..5),
    //             (Ok(Letters::NaConcat), b"éèD", 6..11),
    //             (Ok(Letters::NaRange), b"É", 12..14),
    //             (Err(()), b"è", 14..16),
    //             (Ok(Letters::Altern), b"d", 16..17),
    //             (Ok(Letters::NaAltern), b"Ccû", 18..22),
    //             (Ok(Letters::NaRange), b"Û", 22..24),
    //             (Ok(Letters::NaAltern), b"ü", 24..26),
    //             (Ok(Letters::NaRange), b"Ü", 26..28),
    //             (Ok(Letters::NaAltern), b"C", 28..29),
    //             (Ok(Letters::NaMaybe), b"i", 30..31),
    //             (Ok(Letters::NaMaybe), b"i§", 32..35),
    //             (Ok(Letters::NaRange), b"xXyYzZ|{}", 36..45),
    //         ],
    //     )
    // }
}

mod ignore_case {
    // use logos::Logos as _;
    use logos_derive::Logos;
    use tests::assert_lex;

    #[derive(Logos, Debug, PartialEq, Eq)]
    #[logos(skip " +")]
    enum Words {
        #[token("élÉphAnt", ignore(case))]
        Elephant,
        #[token("ÉlèvE", ignore(case))]
        Eleve,
        #[token("à", ignore(case))]
        A,

        #[token("[abc]+", ignore(case))]
        Abc,
    }

    #[test]
    fn tokens() {
        assert_lex(
            "ÉLÉPHANT Éléphant ÉLèVE à À a",
            &[
                (Ok(Words::Elephant), "ÉLÉPHANT", 0..10),
                (Ok(Words::Elephant), "Éléphant", 11..21),
                (Ok(Words::Eleve), "ÉLèVE", 22..29),
                (Ok(Words::A), "à", 30..32),
                (Ok(Words::A), "À", 33..35),
                (Err(()), "a", 36..37),
            ],
        )
    }

    #[test]
    fn tokens_regex_escaped() {
        assert_lex(
            "[abc]+ abccBA",
            &[
                (Ok(Words::Abc), "[abc]+", 0..6),
                (Err(()), "a", 7..8),
                (Err(()), "b", 8..9),
                (Err(()), "c", 9..10),
                (Err(()), "c", 10..11),
                (Err(()), "B", 11..12),
                (Err(()), "A", 12..13),
            ],
        )
    }

    #[derive(Logos, PartialEq, Eq, Debug)]
    #[logos(skip " +")]
    enum Sink {
        #[regex("[abcéà]+", ignore(case))]
        Letters,
        #[regex("[0-9]+", ignore(case))]
        Numbers,
        #[regex("ééààé", ignore(case))]
        Sequence,
    }

    #[test]
    fn regex() {
        assert_lex(
            "aabbccééààéé 00123 ééààé ABCÉÀÀ ÉÉàÀÉ",
            &[
                (Ok(Sink::Letters), "aabbccééààéé", 0..18),
                (Ok(Sink::Numbers), "00123", 19..24),
                (Ok(Sink::Sequence), "ééààé", 25..35),
                (Ok(Sink::Letters), "ABCÉÀÀ", 36..45),
                (Ok(Sink::Sequence), "ÉÉàÀÉ", 46..56),
            ],
        )
    }
}
