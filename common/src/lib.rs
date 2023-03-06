#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<usize> for Span {
    fn from(index: usize) -> Self {
        Span {
            start: index,
            end: index,
        }
    }
}
