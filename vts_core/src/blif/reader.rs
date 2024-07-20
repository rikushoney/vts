// TODO(rikus): Remove this once everything is implemented.
#![allow(dead_code)]

use std::fmt;
use std::io::Read;

use thiserror::Error;

use super::netlist::Netlist;

use crate::bytescanner::Scanner;

trait BlifChar {
    fn is_line_whitespace(&self) -> bool;

    fn is_token_terminator(&self) -> bool;

    fn is_cube_input(&self) -> bool;

    fn is_cube_output(&self) -> bool;
}

impl BlifChar for u8 {
    /// Returns `true` if the byte is whitespace (excluding newlines),
    /// else `false`.
    #[inline]
    fn is_line_whitespace(&self) -> bool {
        // TODO: Confirm this with other tools.
        matches!(*self, b'\t' | b'\x0C' | b'\r' | b' ')
    }

    /// Returns `true` if the byte would end a single token, else `false`.
    #[inline]
    fn is_token_terminator(&self) -> bool {
        // TODO: Confirm this with other tools.
        matches!(*self, b'\t' | b'\x0C' | b'\r' | b' ' | b'\n' | b'#')
    }

    /// Returns `true` if the byte is a valid cube input value, else `false`.
    #[inline]
    fn is_cube_input(&self) -> bool {
        matches!(*self, b'0' | b'1' | b'-')
    }

    /// Returns `true` if the byte is a valid cube output value, else `false`.
    #[inline]
    fn is_cube_output(&self) -> bool {
        matches!(*self, b'0' | b'1')
    }
}

trait BlifScanner<'a> {
    fn eat_line_whitespace(&mut self) -> &'a [u8];

    fn eat_non_whitespace(&mut self) -> &'a [u8];

    fn eat_token(&mut self) -> &'a [u8];

    fn at_token_terminator(&self) -> bool;
}

impl<'a> BlifScanner<'a> for Scanner<'a> {
    /// Consume whitespace, excluding newlines.
    #[inline]
    fn eat_line_whitespace(&mut self) -> &'a [u8] {
        self.eat_while(BlifChar::is_line_whitespace)
    }

    /// Consume non-whitespace.
    #[inline]
    fn eat_non_whitespace(&mut self) -> &'a [u8] {
        self.eat_until(u8::is_ascii_whitespace)
    }

    /// Consume a single token.
    #[inline]
    fn eat_token(&mut self) -> &'a [u8] {
        self.eat_until(BlifChar::is_token_terminator)
    }

    /// Returns `true` if the scanner is currently at a token terminator, else
    /// `false`.
    fn at_token_terminator(&self) -> bool {
        self.at(BlifChar::is_token_terminator)
    }
}

/// A location in BLIF text/bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    /// Filename, if known.
    pub filename: Option<String>,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column offset.
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO(rikus): Move this out of the fn.
        struct Filename<'a>(&'a str);

        impl fmt::Display for Filename<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.0.chars().any(char::is_whitespace) {
                    write!(formatter, "\"{}\"", self.0)
                } else {
                    write!(formatter, "{}", self.0)
                }
            }
        }

        let filename = self
            .filename
            .as_ref()
            .map(|name| match name.as_str() {
                "-" => "<stdin>",
                name => name,
            })
            .unwrap_or("<unknown>");
        write!(
            formatter,
            "{}:{}:{}",
            Filename(filename),
            self.line,
            self.column
        )
    }
}

/// A parsing error.
#[derive(Clone, Debug, Error)]
pub enum ParseError {
    #[error(r#"unknown directive "{0}""#)]
    UnknownDirective(String),
}

/// A parsing error, tagged with an associated source location.
#[derive(Clone, Debug, Error)]
#[error(
    r#"{error}

while parsing {location}"#
)]
pub struct TaggedParseError {
    error: ParseError,
    location: SourceLocation,
}

/// A reading error.
// TODO(rikus): Merge with blif crate error.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(TaggedParseError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn new_parse<E>(err: E, location: SourceLocation) -> Self
    where
        ParseError: From<E>,
    {
        Self::Parse(TaggedParseError {
            error: ParseError::from(err),
            location,
        })
    }
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

trait ParseLocation<T> {
    fn location(self, location: SourceLocation) -> Result<T>;

    fn with_location<F>(self, make_location: F) -> Result<T>
    where
        F: FnMut() -> SourceLocation;
}

impl<T> ParseLocation<T> for ParseResult<T> {
    /// Tag a `ParseResult` with a location.
    fn location(self, location: SourceLocation) -> Result<T> {
        self.map_err(|error| Error::new_parse(error, location))
    }

    /// Tag a `ParseResult` with a location calculated by `make_location`.
    fn with_location<F>(self, mut make_location: F) -> Result<T>
    where
        F: FnMut() -> SourceLocation,
    {
        self.location(make_location())
    }
}

/// An owned buffer of BLIF text/bytes.
#[derive(Debug, Default)]
pub struct BlifBuffer {
    filename: Option<String>,
    inner: Vec<u8>,
}

/// A buffer byte position.
///
/// NOTE: A `BytePos` should always remain relative to the original buffer extent
/// and never a subslice of it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BytePos(usize);

impl BytePos {
    /// Shift the byte position "up" by `delta`.
    #[must_use]
    fn rebase(mut self, delta: usize) -> Self {
        self.0 += delta;
        self
    }

    /// Get the distance to jump from `other` to `self`.
    ///
    /// Panics if `self` is less than `other`.
    fn diff(&self, other: BytePos) -> usize {
        assert!(self.0 >= other.0);
        self.0 - other.0
    }
}

/// The position of a newline escape character and the associated escaped newline.
#[derive(Debug)]
struct NewlineEscape {
    escape_pos: BytePos,
    newline_pos: BytePos,
}

/// An iterator over logical buffer lines.
#[derive(Debug)]
struct BlifLines<'a> {
    buffer: &'a BlifBuffer,
    line_starts: Vec<BytePos>,
    next_line_i: usize,
    newline_escapes: Vec<NewlineEscape>,
    next_escape_i: usize,
}

impl BlifBuffer {
    /// Create a new buffer with an optional filename.
    pub fn new<I>(bytes: I, filename: Option<String>) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        Self {
            filename,
            inner: Vec::from_iter(bytes),
        }
    }

    /// Create a new buffer by copying a string.
    pub fn new_str(input: &str) -> Self {
        Self::new(input.bytes(), None)
    }

    /// The length of the buffer, in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Create an iterator over the buffer bytes.
    pub fn iter(&self) -> std::slice::Iter<'_, u8> {
        self.inner.iter()
    }

    /// Get a slice of the bytes in `extent`.
    pub fn view(&self, extent: Span) -> &[u8] {
        let end = extent.start_pos.0 + extent.len;
        &self.inner[extent.start_pos.0..end]
    }

    /// Calculate the 1-based line number and column offset at `pos`.
    ///
    /// Panics if `pos` is out of bounds.
    #[cfg(test)]
    fn calculate_location(&self, pos: usize) -> SourceLocation {
        assert!(pos < self.len());
        let line = self.iter().take(pos).filter(|&&b| b == b'\n').count() + 1;
        let column = self
            .iter()
            .rev()
            .skip(self.len() - pos)
            .take_while(|&&b| b != b'\n')
            .count()
            + 1;
        SourceLocation {
            line,
            column,
            filename: self.filename.clone(),
        }
    }

    /// Preprocess the buffer.
    ///
    /// Returns an iterator over the lines of the buffer. Whitespace at
    /// the start of each line is trimmed but a line can end in arbitrary
    /// whitespace (or a comment).
    fn preprocess(&self) -> BlifLines<'_> {
        let mut scanner = Scanner::new(&self.inner);
        let mut line_starts = Vec::new();
        let mut newline_escapes = Vec::new();
        // Prime the first line.
        scanner.eat_whitespace();
        let mut line_start = BytePos(scanner.cursor());
        while !scanner.done() {
            scanner.eat_until((b'#', b'\n', b'\\'));
            if scanner.eat_if(b'#') {
                // Comments do not escape newlines.
                scanner.eat_until(b'\n');
            }
            if scanner.done() || scanner.eat_if(b'\n') {
                line_starts.push(line_start);
                // Prime the next line.
                scanner.eat_whitespace();
                line_start = BytePos(scanner.cursor());
                continue;
            }
            // Check for a potential newline escape.
            let escape_pos = BytePos(scanner.cursor());
            scanner.expect(b'\\');
            // TODO: Should we be more strict about allowed characters between
            // the '\\' and '\n'?
            scanner.eat_line_whitespace();
            let newline_pos = BytePos(scanner.cursor());
            if scanner.eat_if(b'\n') {
                newline_escapes.push(NewlineEscape {
                    escape_pos,
                    newline_pos,
                });
            }
        }
        BlifLines {
            buffer: self,
            line_starts,
            next_line_i: 0,
            newline_escapes,
            next_escape_i: 0,
        }
    }
}

#[derive(Clone, Debug)]
enum LineStorageInner<'a> {
    Owned(Vec<u8>),
    Borrowed(&'a [u8]),
}

/// A copy-on-write reference to borrowed bytes or an owned buffer.
#[derive(Clone, Debug)]
struct LineStorage<'a> {
    inner: LineStorageInner<'a>,
    start_pos: BytePos,
}

impl<'a> LineStorage<'a> {
    /// Create new owned storage.
    fn new_owned<I>(bytes: I, start_pos: BytePos) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        Self {
            inner: LineStorageInner::Owned(Vec::from_iter(bytes)),
            start_pos,
        }
    }

    /// Create new borrowed storage.
    fn new_ref(bytes: &'a [u8], start: BytePos) -> Self {
        Self {
            inner: LineStorageInner::Borrowed(bytes),
            start_pos: start,
        }
    }

    /// The bytes of the line, independent of storage kind.
    fn get_bytes(&self) -> &[u8] {
        match &self.inner {
            LineStorageInner::Owned(owned) => owned,
            LineStorageInner::Borrowed(bytes) => bytes,
        }
    }

    /// Get a mutable reference to the underlying owned storage, if owned.
    ///
    /// NOTE: For a non-`mut` version, use [get_bytes](Self::get_bytes).
    fn get_owned(&mut self) -> Option<&mut Vec<u8>> {
        match self.inner {
            LineStorageInner::Owned(ref mut owned) => Some(owned),
            LineStorageInner::Borrowed(_) => None,
        }
    }

    /// Get a reference to the underlying borrowed bytes, if borrowed.
    fn get_borrowed(&self) -> Option<&[u8]> {
        match self.inner {
            LineStorageInner::Owned(_) => None,
            LineStorageInner::Borrowed(bytes) => Some(bytes),
        }
    }

    /// Copy borrowed bytes to an owned buffer, if borrowed, or do nothing.
    fn make_owned(&mut self) {
        if let Some(bytes) = self.get_borrowed() {
            *self = Self::new_owned(Vec::from_iter(bytes.iter().copied()), self.start_pos);
        }
    }

    /// Invoke the callback with a mutable reference to owned bytes.
    ///
    /// Borrowed bytes are first copied to an owned buffer, which is then passed
    /// to the callback. Owned bytes are passed as is.
    fn make_owned_and<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Vec<u8>),
    {
        match self.inner {
            LineStorageInner::Owned(ref mut owned) => {
                f(owned);
            }
            LineStorageInner::Borrowed(bytes) => {
                let mut bytes = Vec::from_iter(bytes.iter().copied());
                f(&mut bytes);
                *self = Self::new_owned(bytes, self.start_pos);
            }
        }
    }
}

impl AsRef<[u8]> for LineStorage<'_> {
    fn as_ref(&self) -> &[u8] {
        self.get_bytes()
    }
}

// The next line should end at the position of the start of the line
// following the next line. For the final line there is no following line
// and thus the end of the buffer is used.
//               next_line_end
//               |     buffer_len
//               |     |
// "these\n are\n lines"
//         |
//         next_line_start
impl<'a> BlifLines<'a> {
    /// Get the start position of the next line, if any lines remain.
    fn next_line_start(&self) -> Option<BytePos> {
        debug_assert!(self.next_line_i <= self.line_starts.len());
        if self.next_line_i < self.line_starts.len() {
            Some(self.line_starts[self.next_line_i])
        } else {
            None
        }
    }

    /// Get the end position of the next line.
    fn next_line_end(&self) -> BytePos {
        debug_assert!(self.next_line_i <= self.line_starts.len());
        let next_next_line_i = self.next_line_i + 1;
        if next_next_line_i < self.line_starts.len() {
            self.line_starts[next_next_line_i]
        } else {
            BytePos(self.buffer.len())
        }
    }

    fn get_line(&self, start_pos: BytePos, end_pos: BytePos) -> &'a [u8] {
        &self.buffer.inner[start_pos.0..end_pos.0]
    }

    /// Replace escape characters and associated newlines in `storage` with
    /// whitespace.
    ///
    /// `end_pos` marks the end position of the line storage in the buffer.
    fn patch_newline_escapes(&mut self, storage: &mut LineStorage<'_>, end_pos: BytePos) {
        let start_pos = storage.start_pos;
        let remaining_newline_escapes = self.newline_escapes.iter().skip(self.next_escape_i);
        let line_extent = start_pos.0..end_pos.0;
        for &NewlineEscape {
            escape_pos,
            newline_pos,
        } in remaining_newline_escapes
        {
            debug_assert!(escape_pos < newline_pos);
            if line_extent.contains(&newline_pos.0) {
                // Create a copy of the line (if not already owned) and patch
                // the escape character and associated escaped newline.
                storage.make_owned_and(|bytes| {
                    // NOTE: Substract `start_pos` since `escape_pos` and
                    // `newline_pos` is relative to it.
                    bytes[escape_pos.diff(start_pos)] = b' ';
                    bytes[newline_pos.diff(start_pos)] = b' ';
                });
                self.next_escape_i += 1;
            } else {
                break;
            }
        }
    }
}

impl<'a> Iterator for BlifLines<'a> {
    type Item = LineStorage<'a>;

    /// Get the next logical line.
    ///
    /// Escaped newlines and the associated escape characters are replaced by
    /// whitespace.
    fn next(&mut self) -> Option<Self::Item> {
        let next_line_start = self.next_line_start()?;
        let next_line_end = self.next_line_end();
        let next_line = self.get_line(next_line_start, next_line_end);
        let mut storage = LineStorage::new_ref(next_line, next_line_start);
        self.patch_newline_escapes(&mut storage, next_line_end);
        self.next_line_i += 1;
        Some(storage)
    }

    // Due to pre-processing we always know how many lines are left for iteration.
    // Some lines might be empty or comments, though.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.line_starts.len() - self.next_line_i))
    }
}

impl<I> From<I> for BlifBuffer
where
    I: IntoIterator<Item = u8>,
{
    fn from(input: I) -> Self {
        Self {
            filename: None,
            inner: Vec::from_iter(input),
        }
    }
}

/// A buffer extent.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    start_pos: BytePos,
    len: usize,
}

impl Span {
    /// Create a new span.
    fn new(start_pos: BytePos, len: usize) -> Self {
        Self { start_pos, len }
    }

    /// Create a new span at `start` with length `end - start`.
    fn new_range(start_pos: BytePos, end: BytePos) -> Self {
        Self::new(start_pos, end.0 - start_pos.0)
    }

    /// Shift the span start "up" by `delta`.
    #[must_use]
    fn rebase(mut self, delta: usize) -> Self {
        self.start_pos = self.start_pos.rebase(delta);
        self
    }

    /// Create a new span starting at `base` shifted "up" by `start` and length
    /// `end - start`.
    fn new_rebased_range(base: BytePos, start_pos: usize, end_pos: usize) -> Self {
        Span::new(base, end_pos - start_pos).rebase(start_pos)
    }
}

/// The scanned token kind.
#[derive(Debug, PartialEq)]
enum TokenKind {
    /// A command.
    Command,
    /// A cube.
    Cube,
}

// A scanned token.
#[derive(Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    trivia: Vec<Span>,
    extent: Span,
}

/// An iterator over scanned tokens.
struct Tokenizer<'a> {
    lines: BlifLines<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Start a new tokenizer over `lines`.
    fn new(lines: BlifLines<'a>) -> Self {
        Self { lines }
    }

    /// Pre-process and start a new tokenizer.
    fn new_preprocess(buffer: &'a BlifBuffer) -> Self {
        Self::new(buffer.preprocess())
    }
}

/// `Command ::= "." name (S+ arg0 (S+ argn)*)?`
fn tokenize_command_line(line: &[u8], start_pos: BytePos) -> Result<Token> {
    let mut scanner = Scanner::new(line);
    scanner.expect(b'.');
    scanner.eat_token();
    let mut token_end = scanner.cursor();
    if token_end < 2 {
        // TODO(rikus): Report empty command name.
        panic!("empty command name");
    }
    scanner.eat_whitespace();
    // Start `trivia` with a span of the command name.
    let mut trivia = vec![Span::new(start_pos, token_end)];
    let mut trivia_start = scanner.cursor();
    // NOTE: Trivia beyond the command name is assumed to be optional.
    while !scanner.done() && !scanner.at_token_terminator() {
        scanner.eat_token();
        token_end = scanner.cursor();
        trivia.push(Span::new_rebased_range(start_pos, trivia_start, token_end));
        scanner.eat_whitespace();
        trivia_start = scanner.cursor();
    }
    Ok(Token {
        kind: TokenKind::Command,
        trivia,
        extent: Span::new(start_pos, token_end),
    })
}

/// `Cube ::= ("0" | "1" | "-")+ S+ ("0" | "1")`
fn tokenize_cube_line(line: &[u8], start_pos: BytePos) -> Result<Token> {
    let mut scanner = Scanner::new(line);
    scanner.expect(BlifChar::is_cube_input);
    scanner.eat_while(BlifChar::is_cube_input);
    let input_end = scanner.cursor();
    if scanner.eat_whitespace().is_empty() {
        // TODO(rikus): Handle expected whitespace.
        panic!("expected whitespace");
    }
    let output_start = scanner.cursor();
    // NOTE: Multi-bit outputs will be detected as errors by the parsing stage.
    scanner.eat_while(BlifChar::is_cube_output);
    let token_end = scanner.cursor();
    if output_start == token_end {
        // TODO(rikus): Handle empty output.
        panic!("expected '0' or '1'");
    }
    scanner.eat_whitespace();
    if scanner.at(b'#') {
        scanner.jump_end();
    }
    if !scanner.done() {
        // TODO(rikus): Handle unexpected trailing bytes.
        panic!("unexpected {:?}", &line[scanner.cursor()..]);
    }
    Ok(Token {
        kind: TokenKind::Cube,
        trivia: Vec::from_iter([
            Span::new(start_pos, input_end),
            Span::new_rebased_range(start_pos, output_start, token_end),
        ]),
        extent: Span::new(start_pos, token_end),
    })
}

impl Iterator for Tokenizer<'_> {
    type Item = Result<Token>;

    /// Get the next line's tokens from the tokenizer.
    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            let line_bytes = line.get_bytes();
            let mut scanner = Scanner::new(line_bytes);
            match scanner.peek() {
                Some(b'.') => {
                    return Some(tokenize_command_line(line_bytes, line.start_pos));
                }
                Some(logic) if logic.is_cube_input() => {
                    return Some(tokenize_cube_line(line_bytes, line.start_pos));
                }
                Some(b'#') => {
                    // Comment lines are ignored.
                    continue;
                }
                Some(unexpected) => {
                    // TODO: Handle unexpected.
                    panic!("unexpected {:?}", unexpected);
                }
                None => {
                    // TODO: Should this be `unreachable!`?
                    panic!("unexpected empty line");
                }
            }
        }
        None
    }
}

impl BlifBuffer {
    /// Preprocess and create a new [Tokenizer] from the buffer.
    fn tokenize(&self) -> Tokenizer {
        Tokenizer::new_preprocess(self)
    }
}

/// A BLIF file reader.
#[derive(Debug)]
pub struct BlifReader {
    buffer: BlifBuffer,
}

impl BlifReader {
    pub fn from_reader<R: Read>(mut reader: R, filename: Option<&str>) -> Result<Self> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(Self {
            buffer: BlifBuffer {
                filename: filename.map(str::to_string),
                inner: buffer,
            },
        })
    }

    pub fn from_str(input: &str, filename: Option<&str>) -> Self {
        Self {
            buffer: BlifBuffer::new(input.to_string().into_bytes(), filename.map(str::to_string)),
        }
    }

    pub fn parse_netlist(&mut self) -> Result<Netlist> {
        let tokenizer = self.buffer.tokenize();
        let _ = tokenizer.count();
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod source_location {
        use super::*;

        macro_rules! loc {
            ($line:expr, $col:expr) => {
                SourceLocation {
                    line: $line,
                    column: $col,
                    filename: None,
                }
            };
        }

        #[test]
        fn test_calculate_location() {
            let buffer = BlifBuffer::new_str(
                r#".model top
.inputs a b c
.outputs d
.names a b c d
000 1
.end
"#,
            );
            assert_eq!(buffer.calculate_location(0), loc!(1, 1));
            assert_eq!(buffer.calculate_location(9), loc!(1, 10));
            assert_eq!(buffer.calculate_location(10), loc!(1, 11));
            assert_eq!(buffer.calculate_location(11), loc!(2, 1));
            assert_eq!(buffer.calculate_location(25), loc!(3, 1));
            assert_eq!(buffer.len(), 62);
            assert_eq!(buffer.calculate_location(60), loc!(6, 4));

            let buffer = BlifBuffer::new_str("\na");
            assert_eq!(buffer.calculate_location(0), loc!(1, 1));
            assert_eq!(buffer.calculate_location(1), loc!(2, 1));
        }
    }

    mod preprocess {
        use super::*;

        #[test]
        fn test_empty() {
            let buffer = BlifBuffer::new_str("\n");
            let mut lines = buffer.preprocess();
            assert!(lines.next().is_none());

            let buffer = BlifBuffer::new_str("  \n# empty\n  \\\n  ");
            let mut lines = buffer.preprocess();
            assert_eq!(lines.next().unwrap().get_bytes(), b"# empty\n  ");
            assert_eq!(lines.next().unwrap().get_bytes(), b"    ");
            assert!(lines.next().is_none());
        }
    }

    mod tokenizer {
        use super::*;

        macro_rules! span {
            ($start:expr, $len:expr) => {
                Span::new(BytePos($start), $len)
            };
        }

        macro_rules! check_token {
            ($tokens:expr => command [$(($start:expr, $len:expr)),+$(,)?] @ ($tok_start:expr, $tok_len:expr)) => {
                check_token!($tokens => TokenKind::Command [$(($start, $len)),+] @ ($tok_start, $tok_len))
            };
            ($tokens:expr => cube [$(($start:expr, $len:expr)),+$(,)?] @ ($tok_start:expr, $tok_len:expr)) => {
                check_token!($tokens => TokenKind::Cube [$(($start, $len)),+] @ ($tok_start, $tok_len))
            };
            ($tokens:expr => $kind:path [$(($start:expr, $len:expr)),+$(,)?] @ ($tok_start:expr, $tok_len:expr)) => {
                assert_eq!(
                    $tokens.next().unwrap().unwrap(),
                    Token {
                        kind: $kind,
                        trivia: vec![$(span!($start, $len),)+],
                        extent: span!($tok_start, $tok_len)
                    }
                );
            };
        }

        #[test]
        fn test_command() {
            let buffer = BlifBuffer::new_str(".test a1 b2 3");
            let mut tokenizer = buffer.tokenize();
            let test_start = 0;
            let test_end = test_start + b".test".len();
            let test_len = test_end - test_start;
            let a1_start = test_end + 1;
            let a1_end = a1_start + b"a1".len();
            let a1_len = a1_end - a1_start;
            let b2_start = a1_end + 1;
            let b2_end = b2_start + b"b2".len();
            let b2_len = b2_end - b2_start;
            let _3_start = b2_end + 1;
            let _3_end = _3_start + b"3".len();
            let _3_len = _3_end - _3_start;
            check_token!(tokenizer => command
                [
                    (test_start, test_len),
                    (a1_start, a1_len),
                    (b2_start, b2_len),
                    (_3_start, _3_len)
                ]
                @ (0, buffer.len())
            );
        }

        #[test]
        fn test_strange_syntax() {
            let buffer = BlifBuffer::new_str(
                r#".test a b \
c # test \

### BREAK

.test a \
      b \
      c
"#,
            );
            let mut tokenizer = buffer.tokenize();
            let test_start = 0;
            let test_end = test_start + b".test".len();
            let test_len = test_end - test_start;
            let a_start = test_end + 1;
            let a_end = a_start + 1;
            let a_len = a_end - a_start;
            let b_start = a_end + 1;
            let b_end = b_start + 1;
            let b_len = b_end - b_start;
            let c_start = b_end + b" \\\n".len();
            let c_end = c_start + 1;
            let c_len = c_end - c_start;
            check_token!(tokenizer => command
                [
                    (test_start, test_len),
                    (a_start, a_len),
                    (b_start, b_len),
                    (c_start, c_len)
                ]
                @ (test_start, c_end)
            );
            let test_start = c_end + " # test \\\n\n### BREAK\n\n".len();
            let test_end = test_start + b".test".len();
            let a_start = test_end + 1;
            let a_end = a_start + 1;
            let a_len = a_end - a_start;
            let b_start = a_end + b" \\\n      ".len();
            let b_end = b_start + 1;
            let b_len = b_end - b_start;
            let c_start = b_end + b" \\\n      ".len();
            let c_end = c_start + 1;
            let c_len = c_end - c_start;
            check_token!(tokenizer => command
                [
                    (test_start, test_len),
                    (a_start, a_len),
                    (b_start, b_len),
                    (c_start, c_len)
                ]
                @ (test_start, c_end - test_start)
            );
            assert!(tokenizer.next().is_none());
        }
    }
}
