pub trait OptionExt<T> {
    fn insert(&mut self, val: T, err: &str);
}

impl<T> OptionExt<T> for Option<T> {
    fn insert(&mut self, val: T, err: &str) {
        match self {
            Some(_) => panic!("{}", err),
            slot    => *slot = Some(val),
        }
    }
}
