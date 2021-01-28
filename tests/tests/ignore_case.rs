mod ignore_ascii_case {
    use logos::Logos;
    use tests::assert_lex;

    #[derive(Logos, Debug, PartialEq, Eq)]
    enum Words {
        #[error]
        #[regex(" +", logos::skip)]
        Error,

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
                (Words::Lowercase, "LowErcase", 0..9),
                (Words::Or, "or", 10..12),
                (Words::Uppercase, "UppeRCase", 13..22),
                (Words::Colon, ":", 22..23),
                (Words::That, "ThAT", 24..28),
                (Words::Is, "iS", 29..31),
                (Words::The, "tHe", 32..35),
                (Words::Question, "question", 36..44),
            ],
        )
    }

    #[test]
    fn tokens_nonascii() {
        assert_lex(
            "Mon Frère Était lÀ cet Été",
            &[
                (Words::Mon, "Mon", 0..3),
                (Words::Frere, "Frère", 4..10),
                (Words::Etait, "Était", 11..17),
                (Words::Error, "l", 18..19),
                (Words::Error, "À", 19..21),
                (Words::Cet, "cet", 22..25),
                (Words::Error, "É", 26..28),
                (Words::Error, "t", 28..29),
                (Words::Error, "é", 29..31),
            ],
        )
    }

    #[derive(Logos, Debug, PartialEq, Eq)]
    enum Letters {
        #[error]
        #[regex(" +", logos::skip)]
        Error,

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
        #[regex("i§?", ignore(ascii_case))]
        NaMaybe,
        #[regex("[x-à]+", ignore(ascii_case))]
        NaRange,
    }

    #[test]
    fn regex_simple() {
        assert_lex(
            "aA BCbC DdEE fFff g gg hHiIjJkK",
            &[
                (Letters::Single, "a", 0..1),
                (Letters::Single, "A", 1..2),
                (Letters::Concat, "BC", 3..5),
                (Letters::Concat, "bC", 5..7),
                (Letters::Altern, "D", 8..9),
                (Letters::Altern, "d", 9..10),
                (Letters::Altern, "E", 10..11),
                (Letters::Altern, "E", 11..12),
                (Letters::Loop, "fFff", 13..17),
                (Letters::Maybe, "g", 18..19),
                (Letters::Maybe, "gg", 20..22),
                (Letters::Range, "hHiIjJkK", 23..31),
            ],
        )
    }

    #[test]
    fn regex_nonascii() {
        assert_lex(
            "à À éèD Éèd CcûÛüÜC i i§ xXyYzZ|{}",
            &[
                (Letters::NaSingle, "à", 0..2),
                (Letters::NaRange, "À", 3..5),
                (Letters::NaConcat, "éèD", 6..11),
                (Letters::NaRange, "É", 12..14),
                (Letters::Error, "è", 14..16),
                (Letters::Altern, "d", 16..17),
                (Letters::NaAltern, "Ccû", 18..22),
                (Letters::NaRange, "Û", 22..24),
                (Letters::NaAltern, "ü", 24..26),
                (Letters::NaRange, "Ü", 26..28),
                (Letters::NaAltern, "C", 28..29),
                (Letters::NaMaybe, "i", 30..31),
                (Letters::NaMaybe, "i§", 32..35),
                (Letters::NaRange, "xXyYzZ|{}", 36..45),
            ],
        )
    }
}

mod ignore_case {
    use logos::Logos;
    use tests::assert_lex;

    #[derive(Logos, Debug, PartialEq, Eq)]
    enum Words {
        #[error]
        #[regex(" +", logos::skip)]
        Error,

        #[token("élÉphAnt", ignore(case))]
        Elephant,
        #[token("ÉlèvE", ignore(case))]
        Eleve,
        #[token("à", ignore(case))]
        A,
    }

    #[test]
    fn tokens() {
        assert_lex(
            "ÉLÉPHANT Éléphant ÉLèVE à À a",
            &[
                (Words::Elephant, "ÉLÉPHANT", 0..10),
                (Words::Elephant, "Éléphant", 11..21),
                (Words::Eleve, "ÉLèVE", 22..29),
                (Words::A, "à", 30..32),
                (Words::A, "À", 33..35),
                (Words::Error, "a", 36..37),
            ],
        )
    }

    #[derive(Logos, PartialEq, Eq, Debug)]
    enum Sink {
        #[error]
        #[regex(" +", logos::skip)]
        Error,

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
                (Sink::Letters, "aabbccééààéé", 0..18),
                (Sink::Numbers, "00123", 19..24),
                (Sink::Sequence, "ééààé", 25..35),
                (Sink::Letters, "ABCÉÀÀ", 36..45),
                (Sink::Sequence, "ÉÉàÀÉ", 46..56),
            ],
        )
    }
}
