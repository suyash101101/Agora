use sp_trie::error;
use std::fmt;

#[derive(Debug)]
pub struct StdErrorWrapper<T>(pub error::Error<T>);

impl<T> fmt::Display for StdErrorWrapper<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

// Implement std::error::Error to satisfy NodeCodec
impl<T> std::error::Error for StdErrorWrapper<T> where T: std::fmt::Debug {}
