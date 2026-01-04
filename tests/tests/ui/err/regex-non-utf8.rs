use logos::Logos;

#[derive(Logos)]
enum TokenWithXFF {
    #[regex(b"\xFF")]
    NonUtf8,
}

#[derive(Logos)]
enum TokenWithX00Star {
    #[regex(b"\x00.*")]
    NonUtf8,
}

#[derive(Logos)]
enum TokenWithX00Plus {
    #[regex(b"\x00.+")]
    NonUtf8,
}

fn main() {}
