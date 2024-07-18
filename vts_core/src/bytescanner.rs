// A bytes/ascii port of [unscanny](https://github.com/typst/unscanny).

use std::fmt;
use std::ops::Range;

mod sealed {
    pub trait Sealed<T> {
        fn matches(&mut self, bytes: &[u8]) -> Option<usize>;

        fn expect(&self);
    }
}

use sealed::Sealed;

impl Sealed<()> for u8 {
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        bytes.starts_with(&[*self]).then_some(1)
    }

    #[cold]
    fn expect(&self) {
        panic!("bytes after cursor should be {self:?}");
    }
}

impl Sealed<()> for &[u8] {
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        bytes.starts_with(self).then_some(self.len())
    }

    #[cold]
    fn expect(&self) {
        panic!("bytes after cursor should be {self:?}");
    }
}

impl<const N: usize> Sealed<()> for [u8; N] {
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        self.as_slice().matches(bytes)
    }

    #[cold]
    fn expect(&self) {
        self.as_slice().expect()
    }
}

impl<const N: usize> Sealed<()> for &[u8; N] {
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        self.as_slice().matches(bytes)
    }

    #[cold]
    fn expect(&self) {
        self.as_slice().expect()
    }
}

impl<F> Sealed<u8> for F
where
    F: FnMut(u8) -> bool,
{
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        bytes.iter().next().filter(|&b| self(*b)).map(|_| 1)
    }

    #[cold]
    fn expect(&self) {
        panic!("closure should return `true`")
    }
}

impl<F> Sealed<&u8> for F
where
    F: FnMut(&u8) -> bool,
{
    #[inline]
    fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
        bytes.iter().next().filter(|&b| self(b)).map(|_| 1)
    }

    #[cold]
    fn expect(&self) {
        panic!("closure should return `true`")
    }
}

/// Something a blob of bytes can start with.
pub trait Pattern<T>: sealed::Sealed<T> {}

impl Pattern<()> for u8 {}

impl Pattern<()> for &[u8] {}

impl<const N: usize> Pattern<()> for [u8; N] {}

impl<const N: usize> Pattern<()> for &[u8; N] {}

impl<F> Pattern<u8> for F where F: FnMut(u8) -> bool {}

impl<F> Pattern<&u8> for F where F: FnMut(&u8) -> bool {}

macro_rules! impl_tuple_pattern {
    ($($idx:tt $type:tt),+) => {
        impl Sealed<()> for ($($type,)+) {
            #[inline]
            fn matches(&mut self, bytes: &[u8]) -> Option<usize> {
                let byte = *bytes.iter().next()?;
                [$(self.$idx,)+].iter().any(|&b| b == byte).then_some(1)
            }

            #[cold]
            fn expect(&self) {
                struct Or<'a>(&'a [u8]);

                impl fmt::Debug for Or<'_> {
                    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        let mut iter = self.0.iter();
                        let byte = iter.next().expect("tuple pattern should have at least one element");
                        byte.fmt(formatter)?;
                        for byte in iter {
                            formatter.write_str(" or ")?;
                            byte.fmt(formatter)?;
                        }
                        Ok(())
                    }
                }

                panic!("bytes after cursor should be {:?}", Or(&[$(self.$idx,)+]))
            }
        }

        impl Pattern<()> for ($($type,)+) {}
    }
}

impl_tuple_pattern!(0 u8, 1 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8, 3 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8, 3 u8, 4 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8, 3 u8, 4 u8, 5 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8, 3 u8, 4 u8, 5 u8, 6 u8);
impl_tuple_pattern!(0 u8, 1 u8, 2 u8, 3 u8, 4 u8, 5 u8, 6 u8, 7 u8);

/// A byte scanner.
pub struct Scanner<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Scanner<'a> {
    /// Create a new byte scanner with a cursor position of `0`.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    /// The entire byte slice.
    #[inline]
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// The current position of the cursor.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether the scanner has fully consumed the bytes.
    #[inline]
    pub fn done(&self) -> bool {
        self.cursor == self.bytes.len()
    }

    /// The subslice of bytes before the cursor.
    #[inline]
    pub fn before(&self) -> &'a [u8] {
        &self.bytes[..self.cursor]
    }

    /// The subslice of bytes after the cursor.
    #[inline]
    pub fn after(&self) -> &'a [u8] {
        &self.bytes[self.cursor..]
    }

    /// The subslices before and after the cursor.
    #[inline]
    pub fn parts(&self) -> (&'a [u8], &'a [u8]) {
        (self.before(), self.after())
    }

    /// The subslice from `start` to the current cursor position.
    ///
    /// - If `start` is greater than or equal to the current cursor position,
    ///   an empty slice is returned.
    #[inline]
    pub fn from(&self, start: usize) -> &'a [u8] {
        let start = start.min(self.cursor);
        &self.bytes[start..self.cursor]
    }

    /// The subslice from the current cursor position to `end`.
    ///
    /// - If `end` is less than or equal to the current cursor position, an
    ///   empty slice is returned.
    /// - If `end` is greater than or equal to the length of the bytes, a
    ///   slice of all the bytes starting from the current cursor position is
    ///   returned.
    #[inline]
    pub fn to(&self, end: usize) -> &'a [u8] {
        let end = end.max(self.cursor).min(self.bytes.len());
        &self.bytes[self.cursor..end]
    }

    /// The subslice from `range.start` to `range.end`.
    ///
    /// - If `range.end` is less than or equal to `range.start`, an empty
    ///   slice is returned.
    /// - If `range.end` is greater than or equal to the length of the bytes,
    ///   a slice of all of the bytes starting from `range.start` is returned.
    #[inline]
    pub fn get(&self, range: Range<usize>) -> &'a [u8] {
        let end = range.end.min(self.bytes.len());
        let start = range.start.min(end);
        &self.bytes[start..end]
    }

    /// Peek the byte right at the cursor.
    #[inline]
    pub fn peek(&mut self) -> Option<u8> {
        self.after().iter().next().copied()
    }

    /// Returns `true` if the bytes right after the cursor match `pattern`,
    /// else `false`.
    #[inline]
    pub fn at<T>(&self, mut pattern: impl Pattern<T>) -> bool {
        pattern.matches(self.after()).is_some()
    }

    /// Peek at the n-th byte relative to the current cursor position.
    #[inline]
    pub fn scout(&self, n: isize) -> Option<u8> {
        if n >= 0 {
            self.after().iter().nth(n as usize).copied()
        } else {
            self.before().iter().nth_back((-n - 1) as usize).copied()
        }
    }

    /// Consume a single byte.
    ///
    /// Returns the consumed byte if there are any bytes left.
    #[inline]
    pub fn eat(&mut self) -> Option<u8> {
        let peeked = self.peek();
        if peeked.is_some() {
            self.cursor += 1;
        }
        peeked
    }

    /// Un-consume a single byte.
    ///
    /// Returns the un-consumed byte if the cursor is not at the start of the
    /// bytes.
    #[inline]
    pub fn uneat(&mut self) -> Option<u8> {
        let unpeeked = self.before().iter().next_back().copied();
        if unpeeked.is_some() {
            self.cursor -= 1;
        }
        unpeeked
    }

    /// Consume bytes if they match `pattern`.
    ///
    /// Returns `true` if bytes were consumed else `false`.
    #[inline]
    pub fn eat_if<T>(&mut self, mut pattern: impl Pattern<T>) -> bool {
        if let Some(len) = pattern.matches(self.after()) {
            self.cursor += len;
            true
        } else {
            false
        }
    }

    /// Consume bytes while they match `pattern`.
    ///
    /// Returns a slice of all the consumed bytes.
    #[inline]
    pub fn eat_while<T>(&mut self, mut pattern: impl Pattern<T>) -> &'a [u8] {
        let start = self.cursor;
        while let Some(len @ 1..) = pattern.matches(self.after()) {
            self.cursor += len;
        }
        self.from(start)
    }

    /// Consume bytes until the next bytes match `pattern`.
    ///
    /// Returns a slice of all the consumed bytes.
    #[inline]
    pub fn eat_until<T>(&mut self, mut pattern: impl Pattern<T>) -> &'a [u8] {
        let start = self.cursor;
        while !self.done() && pattern.matches(self.after()).is_none() {
            self.cursor += 1;
        }
        self.from(start)
    }

    /// Consume bytes until the next bytes match `pattern`.
    ///
    /// Returns a slice of all the consumed bytes and the terminating bytes.
    #[inline]
    pub fn eat_until_terminator<T>(&mut self, mut pattern: impl Pattern<T>) -> &'a [u8] {
        let start = self.cursor;
        while !self.done() && pattern.matches(self.after()).is_none() {
            self.cursor += 1;
        }
        if let Some(len) = pattern.matches(self.after()) {
            self.cursor += len;
        }
        self.from(start)
    }

    /// Consume whitespace bytes until the next non-whitespace byte.
    ///
    /// Returns a slice of the consumed whitespace.
    #[inline]
    pub fn eat_whitespace(&mut self) -> &'a [u8] {
        self.eat_while(u8::is_ascii_whitespace)
    }

    /// Consume bytes if they match `pattern` or panic.
    #[inline]
    pub fn expect<T>(&mut self, mut pattern: impl Pattern<T>) {
        if let Some(len) = pattern.matches(self.after()) {
            self.cursor += len;
        } else {
            pattern.expect();
        }
    }

    #[inline]
    /// Jump the cursor position to `cursor`.
    pub fn jump(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.bytes.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let mut s = Scanner::new(&[]);
        s.jump(10);
        assert_eq!(s.cursor(), 0);
        assert_eq!(s.done(), true);
        assert_eq!(s.before(), b"");
        assert_eq!(s.after(), b"");
        assert_eq!(s.from(0), b"");
        assert_eq!(s.to(10), b"");
        assert_eq!(s.get(10..20), b"");
        assert_eq!(s.at(|_| true), false);
        assert_eq!(s.at(b""), true);
        assert_eq!(s.at(b'a'), false);
        assert_eq!(s.scout(-1), None);
        assert_eq!(s.scout(1), None);
        assert_eq!(s.eat(), None);
        assert_eq!(s.uneat(), None);
        assert_eq!(s.eat_if(b""), true);
        assert_eq!(s.eat_if(b'a'), false);
        assert_eq!(s.eat_while(b""), b"");
        assert_eq!(s.eat_while(b'a'), b"");
        assert_eq!(s.eat_until(b""), b"");
        assert_eq!(s.eat_whitespace(), b"");
    }

    #[test]
    fn test_multiple() {
        let mut s = Scanner::new(b"abc");
        let pat = (b'a', b'b');
        s.expect(pat);
        s.expect(pat);
        assert_eq!(s.at(pat), false);
    }
}
