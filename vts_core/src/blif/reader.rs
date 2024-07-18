// TODO(rikus): Remove this once everything is implemented.
#![allow(dead_code)]

use std::fmt;
use std::io::Read;

use thiserror::Error;

use super::netlist::Netlist;

use crate::bytescanner::Scanner;

trait BlifChar {
    fn is_line_whitespace(&self) -> bool;
}

impl BlifChar for u8 {
    /// Returns `true` if the byte is whitespace (excluding newlines),
    /// else `false`.
    #[inline]
    fn is_line_whitespace(&self) -> bool {
        matches!(*self, b'\t' | b'\x0C' | b'\r' | b' ')
    }
}

trait BlifScanner<'a> {
    fn eat_line_whitespace(&mut self) -> &'a [u8];

    fn eat_until_whitespace(&mut self) -> &'a [u8];
}

impl<'a> BlifScanner<'a> for Scanner<'a> {
    fn eat_line_whitespace(&mut self) -> &'a [u8] {
        self.eat_while(BlifChar::is_line_whitespace)
    }

    fn eat_until_whitespace(&mut self) -> &'a [u8] {
        self.eat_until(u8::is_ascii_whitespace)
    }
}

/// A location in BLIF text/bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceLocation {
    /// Filename, if known.
    pub file: Option<String>,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column offset.
    pub column: usize,
}

impl Eq for SourceLocation {}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            .file
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
    inner: Box<[u8]>,
}

/// An escape offset/newline offset pair.
#[derive(Debug)]
struct NewlineEscape {
    escape_offset: usize,
    newline_offset: usize,
}

/// An iterator over logical buffer lines.
#[derive(Debug)]
struct BlifLines<'a> {
    buffer: &'a BlifBuffer,
    line_offsets: Box<[usize]>,
    next_line_i: usize,
    newline_escapes: Box<[NewlineEscape]>,
    next_escape_i: usize,
}

impl BlifBuffer {
    /// Create a new buffer with an optional filename.
    fn new<I>(bytes: I, filename: Option<String>) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        Self {
            filename,
            inner: Box::from_iter(bytes),
        }
    }

    /// Create a new buffer by copying a string.
    #[cfg(test)]
    fn new_str(input: &str) -> Self {
        Self::new(input.bytes(), None)
    }

    /// The length of the buffer, in bytes.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// View the bytes in `extent`.
    pub fn view(&self, extent: Span) -> &[u8] {
        let end = extent.start + extent.len;
        &self.inner[extent.start..end]
    }

    /// Calculate the 1-based line number and column offset at `offset`.
    ///
    /// Panics if `offset` is out of bounds.
    #[cfg(test)]
    fn calculate_location(&self, offset: usize) -> SourceLocation {
        assert!(offset < self.inner.len());
        let line = self
            .inner
            .iter()
            .take(offset)
            .filter(|&&b| b == b'\n')
            .count()
            + 1;
        let column = self
            .inner
            .iter()
            .rev()
            .skip(self.inner.len() - offset)
            .take_while(|&&b| b != b'\n')
            .count()
            + 1;
        SourceLocation {
            line,
            column,
            file: self.filename.clone(),
        }
    }

    /// Preprocess the buffer.
    ///
    /// Returns an iterator over the lines of the buffer. Whitespace at the
    /// start of each line is trimmed but a line can end in whitespace.
    fn preprocess(&self) -> BlifLines<'_> {
        let mut scanner = Scanner::new(&self.inner);
        let mut line_offsets = Vec::new();
        let mut newline_escapes = Vec::new();
        // Prime the first line.
        scanner.eat_whitespace();
        let mut line_start = scanner.cursor();
        while !scanner.done() {
            scanner.eat_until((b'\n', b'\\', b'#'));
            if scanner.eat_if(b'#') {
                // Comments do not escape newlines.
                scanner.eat_until(b'\n');
            }
            if scanner.done() || scanner.eat_if(b'\n') {
                line_offsets.push(line_start);
                // Prime the next line.
                scanner.eat_whitespace();
                line_start = scanner.cursor();
                continue;
            }
            // Check for a potential newline escape.
            let escape_offset = scanner.cursor();
            scanner.expect(b'\\');
            // TODO: Should we be more strict about allowed characters between
            // the '\\' and '\n'?
            scanner.eat_line_whitespace();
            let newline_offset = scanner.cursor();
            if scanner.eat_if(b'\n') {
                newline_escapes.push(NewlineEscape {
                    escape_offset,
                    newline_offset,
                });
            }
        }
        BlifLines {
            buffer: self,
            line_offsets: Box::from_iter(line_offsets),
            next_line_i: 0,
            newline_escapes: Box::from_iter(newline_escapes),
            next_escape_i: 0,
        }
    }
}

#[derive(Clone, Debug)]
enum LineStorageInner<'a> {
    Owned(Box<[u8]>),
    Borrowed(&'a [u8]),
}

/// A copy-on-write reference to owned or borrowed bytes.
#[derive(Clone, Debug)]
struct LineStorage<'a> {
    inner: LineStorageInner<'a>,
    start_offset: usize,
}

impl<'a> LineStorage<'a> {
    /// Create new owned storage.
    fn new_owned(owned: Box<[u8]>, start_offset: usize) -> Self {
        Self {
            inner: LineStorageInner::Owned(owned),
            start_offset,
        }
    }

    /// Create new borrowed storage.
    fn new_ref(bytes: &'a [u8], start_offset: usize) -> Self {
        Self {
            inner: LineStorageInner::Borrowed(bytes),
            start_offset,
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
    fn get_owned(&mut self) -> Option<&mut Box<[u8]>> {
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
            *self = Self::new_owned(Box::from_iter(bytes.iter().copied()), self.start_offset);
        }
    }

    /// Invoke the callback with a mutable reference to owned bytes.
    ///
    /// Borrowed bytes are first copied to an owned buffer, which is then passed
    /// to the callback. Owned bytes are passed as is.
    fn make_owned_and<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Box<[u8]>),
    {
        match self.inner {
            LineStorageInner::Owned(ref mut owned) => {
                f(owned);
            }
            LineStorageInner::Borrowed(bytes) => {
                let mut bytes = Box::from_iter(bytes.iter().copied());
                f(&mut bytes);
                *self = Self::new_owned(bytes, self.start_offset);
            }
        }
    }
}

impl AsRef<[u8]> for LineStorage<'_> {
    fn as_ref(&self) -> &[u8] {
        self.get_bytes()
    }
}

impl<'a> Iterator for BlifLines<'a> {
    type Item = LineStorage<'a>;

    /// Get the next logical line.
    ///
    /// Escaped newlines and the associated escape characters are replaced by
    /// whitespace.
    fn next(&mut self) -> Option<Self::Item> {
        debug_assert!(self.next_line_i <= self.line_offsets.len());
        if self.next_line_i == self.line_offsets.len() {
            return None;
        }
        let next_line_start = self.line_offsets[self.next_line_i];
        // The next line should end at the offset to the start of the line
        // following the next line. For the final line there is no following line
        // and thus the end of the buffer is used.
        let next_line_end = self
            .line_offsets
            .get(self.next_line_i + 1)
            .copied()
            .unwrap_or(self.buffer.len());
        let next_line = &self.buffer.inner[next_line_start..next_line_end];
        let mut storage = LineStorage::new_ref(next_line, next_line_start);
        // Check for newline escapes in the current line.
        for &NewlineEscape {
            escape_offset,
            newline_offset,
        } in self.newline_escapes.iter().skip(self.next_escape_i)
        {
            if (next_line_start..next_line_end).contains(&newline_offset) {
                // Create a copy of the line (if not owned already) and patch
                // the escape character and the escaped newline.
                storage.make_owned_and(|bytes| {
                    bytes[escape_offset - next_line_start] = b' ';
                    bytes[newline_offset - next_line_start] = b' ';
                });
                self.next_escape_i += 1;
            } else {
                break;
            }
        }
        self.next_line_i += 1;
        Some(storage)
    }

    /// Due to pre-processing we always know how many lines are left for iteration.
    /// Some lines might be empty or comments, though.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.line_offsets.len() - self.next_line_i))
    }
}

impl<I> From<I> for BlifBuffer
where
    I: IntoIterator<Item = u8>,
{
    fn from(input: I) -> Self {
        Self {
            filename: None,
            inner: Box::from_iter(input),
        }
    }
}

/// An extent of the buffer.
#[derive(Debug, PartialEq)]
pub struct Span {
    start: usize,
    len: usize,
}

impl Span {
    /// Create a new span.
    fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    /// Create a new span at `start` with length `end - start`.
    fn new_range(start: usize, end: usize) -> Self {
        Self::new(start, end - start)
    }

    /// Shift the span start by `delta`.
    fn rebase(mut self, delta: usize) -> Self {
        self.start += delta;
        self
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
    trivia: Box<[Span]>,
    extent: Span,
}

/// An iterator over scanned tokens.
struct Tokenizer<'a> {
    lines: BlifLines<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Start a new tokenizer.
    fn new(lines: BlifLines<'a>) -> Self {
        Self { lines }
    }

    /// Pre-process and start a new tokenizer.
    fn new_preprocess(buffer: &'a BlifBuffer) -> Self {
        Self::new(buffer.preprocess())
    }
}

/// Command ::= "." name (S+ arg0 (S+ argn)*)?
fn tokenize_command_line(line: &[u8], start_offset: usize) -> Result<Token> {
    let mut scanner = Scanner::new(line);
    scanner.expect(b'.');
    scanner.eat_until_whitespace();
    let mut token_end = scanner.cursor();
    if token_end < 2 {
        // TODO(rikus): Report empty command name.
        panic!("empty command name");
    }
    scanner.eat_whitespace();
    // Start `trivia` with a span of the command name.
    let mut trivia = vec![Span::new(start_offset, token_end)];
    let mut trivia_start = scanner.cursor();
    // NOTE: Trivia beyond the command name is assumed to be optional.
    while !scanner.done() {
        scanner.eat_until_whitespace();
        token_end = scanner.cursor();
        trivia.push(Span::new(
            start_offset + trivia_start,
            token_end - trivia_start,
        ));
        scanner.eat_whitespace();
        trivia_start = scanner.cursor();
    }
    Ok(Token {
        kind: TokenKind::Command,
        trivia: Box::from_iter(trivia),
        extent: Span::new(start_offset, token_end),
    })
}

/// Cube ::= ("0" | "1" | "-")+ S+ ("0" | "1")
fn tokenize_cube_line(line: &[u8], start_offset: usize) -> Result<Token> {
    let valid_input = (b'0', b'1', b'-');
    let valid_output = (b'0', b'1');
    let mut scanner = Scanner::new(line);
    // NOTE: This function is only called after encountering a valid cube input.
    // This implies that there will _always_ be at least a single input -- no
    // check for empty input necessary.
    debug_assert!(scanner.at(valid_input));
    scanner.eat_while(valid_input);
    let input_end = scanner.cursor();
    if scanner.eat_whitespace().is_empty() {
        // TODO(rikus): Handle expected whitespace.
        panic!("expected whitespace");
    }
    let output_start = scanner.cursor();
    // NOTE: Multi-bit outputs will be detected as errors by the parsing stage.
    scanner.eat_while(valid_output);
    let token_end = scanner.cursor();
    if output_start == token_end {
        // TODO(rikus): Handle empty output.
        panic!("expected '0' or '1'");
    }
    scanner.eat_whitespace();
    if !scanner.done() {
        // TODO(rikus): Handle unexpected trailing.
        panic!("unexpected {:?}", &line[scanner.cursor()..]);
    }
    Ok(Token {
        kind: TokenKind::Cube,
        trivia: Box::from_iter([
            Span::new(start_offset, input_end),
            Span::new(output_start, token_end - output_start).rebase(start_offset),
        ]),
        extent: Span::new(start_offset, token_end),
    })
}

impl Iterator for Tokenizer<'_> {
    type Item = Result<Token>;

    /// Get the next line's tokens from the tokenizer.
    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            // Trim comments.
            let window_end = match line.get_bytes().iter().position(|&b| b == b'#') {
                Some(end) => end,
                None => line.get_bytes().len(),
            };
            let window = &line.get_bytes()[0..window_end];
            let mut scanner = Scanner::new(window);
            match scanner.peek() {
                Some(b'.') => {
                    return Some(tokenize_command_line(window, line.start_offset));
                }
                Some(_logic @ (b'0' | b'1' | b'-')) => {
                    return Some(tokenize_cube_line(window, line.start_offset));
                }
                None => {
                    // Empty lines are ignored.
                    continue;
                }
                Some(unexpected) => {
                    // TODO: Handle unexpected.
                    panic!("unexpected {:?}", unexpected);
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
                inner: Box::from_iter(buffer),
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
                    file: None,
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
                Span::new($start, $len)
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
                        trivia: Box::from_iter([$(span!($start, $len),)+]),
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
