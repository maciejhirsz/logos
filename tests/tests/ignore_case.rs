mod ignore_ascii_case {
    use logos_derive::Logos;
    use tests::assert_lex;

    #[derive(Logos, Debug, PartialEq, Eq)]
    #[logos(skip " +")]
    enum Words {
        #[token("lOwERCaSe", ignore(ascii_case))]
        Lowercase,
        #[token("or", ignore(ascii_case))]
        Or,
        #[token("UppeRcaSE", ignore(ascii_case))]
        Uppercase,
        #[token(":", ignore(ascii_case))]
        Colon,
        #[token("ThAT", ignore(ascii_case))]
        That,
        #[token("IS", ignore(ascii_case))]
        Is,
        #[token("the", ignore(ascii_case))]
        The,
        #[token("QuEsTiOn", ignore(ascii_case))]
        Question,

        #[token("MON", ignore(ascii_case))]
        Mon,
        #[token("frèRE", ignore(ascii_case))]
        Frere,
        #[token("ÉTAIT", ignore(ascii_case))]
        Etait,
        #[token("là", ignore(ascii_case))]
        La,
        #[token("cET", ignore(ascii_case))]
        Cet,
        #[token("éTé", ignore(ascii_case))]
        Ete,
    }

    #[test]
    fn tokens_simple() {
        assert_lex(
            "LowErcase or UppeRCase: ThAT iS tHe question",
            &[
                (Ok(Words::Lowercase), "LowErcase", 0..9),
                (Ok(Words::Or), "or", 10..12),
                (Ok(Words::Uppercase), "UppeRCase", 13..22),
                (Ok(Words::Colon), ":", 22..23),
                (Ok(Words::That), "ThAT", 24..28),
                (Ok(Words::Is), "iS", 29..31),
                (Ok(Words::The), "tHe", 32..35),
                (Ok(Words::Question), "question", 36..44),
            ],
        )
    }

    #[test]
    fn tokens_nonascii() {
        assert_lex(
            "Mon Frère Était lÀ cet Été",
            &[
                (Ok(Words::Mon), "Mon", 0..3),
                (Ok(Words::Frere), "Frère", 4..10),
                (Ok(Words::Etait), "Était", 11..17),
                (Err(()), "l", 18..19),
                (Err(()), "À", 19..21),
                (Ok(Words::Cet), "cet", 22..25),
                (Err(()), "É", 26..28),
                (Err(()), "t", 28..29),
                (Err(()), "é", 29..31),
            ],
        )
    }

    #[derive(Logos, Debug, PartialEq, Eq)]
    #[logos(skip " +")]
    enum Letters {
        #[regex("a", ignore(ascii_case))]
        Single,
        #[regex("bc", ignore(ascii_case))]
        Concat,
        #[regex("[de]", ignore(ascii_case))]
        Altern,
        #[regex("f+", ignore(ascii_case))]
        Loop,
        #[regex("gg?", ignore(ascii_case))]
        Maybe,
        #[regex("[h-k]+", ignore(ascii_case))]
        Range,

        #[regex("à", ignore(ascii_case))]
        NaSingle,
        #[regex("éèd", ignore(ascii_case))]
        NaConcat,
        #[regex("[cûü]+", ignore(ascii_case))]
        NaAltern,
        #[regex("i§?", priority = 3, ignore(ascii_case))]
        NaMaybe,
        #[regex("[x-à]+", ignore(ascii_case))]
        NaRange,
    }

    #[test]
    fn regex_simple() {
        assert_lex(
            "aA BCbC DdEE fFff g gg hHiIjJkK",
            &[
                (Ok(Letters::Single), "a", 0..1),
                (Ok(Letters::Single), "A", 1..2),
                (Ok(Letters::Concat), "BC", 3..5),
                (Ok(Letters::Concat), "bC", 5..7),
                (Ok(Letters::Altern), "D", 8..9),
                (Ok(Letters::Altern), "d", 9..10),
                (Ok(Letters::Altern), "E", 10..11),
                (Ok(Letters::Altern), "E", 11..12),
                (Ok(Letters::Loop), "fFff", 13..17),
                (Ok(Letters::Maybe), "g", 18..19),
                (Ok(Letters::Maybe), "gg", 20..22),
                (Ok(Letters::Range), "hHiIjJkK", 23..31),
            ],
        )
    }

    #[test]
    fn regex_nonascii() {
        assert_lex(
            "à À éèD Éèd CcûÛüÜC i i§ xXyYzZ|{}",
            &[
                (Ok(Letters::NaSingle), "à", 0..2),
                (Ok(Letters::NaRange), "À", 3..5),
                (Ok(Letters::NaConcat), "éèD", 6..11),
                (Ok(Letters::NaRange), "É", 12..14),
                (Err(()), "è", 14..16),
                (Ok(Letters::Altern), "d", 16..17),
                (Ok(Letters::NaAltern), "Ccû", 18..22),
                (Ok(Letters::NaRange), "Û", 22..24),
                (Ok(Letters::NaAltern), "ü", 24..26),
                (Ok(Letters::NaRange), "Ü", 26..28),
                (Ok(Letters::NaAltern), "C", 28..29),
                (Ok(Letters::NaMaybe), "i", 30..31),
                (Ok(Letters::NaMaybe), "i§", 32..35),
                (Ok(Letters::NaRange), "xXyYzZ|{}", 36..45),
            ],
        )
    }
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
