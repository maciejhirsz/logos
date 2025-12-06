macro_rules! debug {
    ($($arg:tt)*) => {
        if cfg!(feature = "debug") {
            eprint!("[{}:{}:{}] ", file!(), line!(), column!());
            eprintln!($($arg)*)
        }
    }
}
