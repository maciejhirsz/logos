#[cfg(test)]
#[macro_export]
macro_rules! pat {
    ($($r:expr),*) => {vec![$($r.into()),*]};
}

pub type Pattern = Vec<Range>;

#[cfg_attr(test, derive(PartialEq))]
pub struct Range(pub u8, pub u8);

impl From<u8> for Range {
    fn from(byte: u8) -> Range {
        Range(byte, byte)
    }
}
