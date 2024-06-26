use std::ops::Range;

pub const fn index_range<T, const N: usize>(_arr: &[T; N]) -> Range<usize> {
    Range { start: 0, end: N }
}
