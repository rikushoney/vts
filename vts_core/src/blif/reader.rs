use std::fmt;
use std::io::Read;

use thiserror::Error;

use super::netlist::Netlist;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceLocation {
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
}

impl Eq for SourceLocation {}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filename = self
            .file
            .as_ref()
            .map(|name| match name.as_str() {
                "-" => "<stdin>",
                name => name,
            })
            .unwrap_or("<unknown>");
        if filename.bytes().any(|b| b.is_ascii_whitespace()) {
            write!(formatter, "\"{}\":{}:{}", filename, self.line, self.column)
        } else {
            write!(formatter, "{}:{}:{}", filename, self.line, self.column)
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum SyntaxError {
    #[error("expected escaped character after '\\'")]
    InvalidEscape,
}

#[derive(Clone, Debug, Error)]
pub enum ParseError {
    #[error(r#"unknown directive "{0}""#)]
    UnknownDirective(String),
    #[error(transparent)]
    Syntax(#[from] SyntaxError),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        r#"{error}

while parsing {location}"#
    )]
    Parse {
        error: ParseError,
        location: SourceLocation,
    },
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl Error {
    fn new_parse<E>(err: E, location: SourceLocation) -> Self
    where
        ParseError: From<E>,
    {
        Self::Parse {
            error: ParseError::from(err),
            location,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type ParseResult<T> = std::result::Result<T, ParseError>;

pub(super) trait ParseLocation<T> {
    fn location(self, location: SourceLocation) -> Result<T>;

    fn with_location<F>(self, make_location: F) -> Result<T>
    where
        F: FnMut() -> SourceLocation;
}

impl<T> ParseLocation<T> for ParseResult<T> {
    fn location(self, location: SourceLocation) -> Result<T> {
        let tag_location = |error| Error::Parse { error, location };
        self.map_err(tag_location)
    }

    fn with_location<F>(self, mut make_location: F) -> Result<T>
    where
        F: FnMut() -> SourceLocation,
    {
        self.location(make_location())
    }
}

pub enum Directive {
    Model,
    Inputs,
    Outputs,
    Names,
    Latch,
    Subckt,
}

#[derive(Default)]
struct BlifBuffer {
    filename: Option<String>,
    inner: Vec<u8>,
}

impl BlifBuffer {
    fn new<I>(bytes: I, filename: Option<String>) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        Self {
            filename,
            inner: bytes.into_iter().collect(),
        }
    }

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
}

impl From<String> for BlifBuffer {
    fn from(input: String) -> Self {
        Self::new(input.into_bytes(), None)
    }
}

impl From<Vec<u8>> for BlifBuffer {
    fn from(input: Vec<u8>) -> Self {
        Self {
            filename: None,
            inner: input,
        }
    }
}

impl From<&str> for BlifBuffer {
    fn from(input: &str) -> Self {
        Self::new(input.bytes(), None)
    }
}

struct Tokenizer<'a> {
    buffer: &'a BlifBuffer,
    cursor: usize,
}

#[derive(Debug, PartialEq)]
enum Token {
    Newline(usize),
    Ident { offset: usize, len: usize },
}

impl<'a> Tokenizer<'a> {
    fn new(buffer: &'a BlifBuffer) -> Self {
        Self { buffer, cursor: 0 }
    }

    fn bump(&mut self) {
        if self.cursor < self.buffer.inner.len() {
            self.cursor += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        debug_assert!(self.cursor <= self.buffer.inner.len());
        self.buffer.inner.get(self.cursor).copied()
    }

    fn peek_unchecked(&mut self) -> u8 {
        self.buffer.inner.as_slice()[self.cursor]
    }

    fn rewind(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    fn eat_whitespace(&mut self) {
        while let Some(space) = self.peek() {
            if !space.is_ascii_whitespace() {
                break;
            }
            self.bump();
        }
    }

    fn parse_ident(&mut self) -> Token {
        debug_assert!(!self.peek_unchecked().is_ascii_whitespace());
        let start = self.cursor;
        while let Some(byte) = self.peek() {
            // TODO(rikus): specify which characters are valid identifiers
            // rather than just "not whitespace".
            if byte.is_ascii_whitespace() {
                break;
            }
            self.bump();
        }
        let len = self.cursor - start;
        Token::Ident { offset: start, len }
    }

    fn next_byte_escape(&mut self) -> Option<Result<Token>> {
        // We need to support skipping over newline escapes, i.e., "\\\n". It
        // may, however, also be present in different forms such as "\\\r\n" or
        // even with arbitrary space after the "\\", e.g., "\\ \t\r\n". The "\r"
        // and "\n" might even be swapped around for some weird reason...
        // NOTE(rikus): the initial BLIF spec asserts that no whitespace should
        // follow the '\\' -- maybe issue a warning if this is detected.
        debug_assert!(matches!(self.buffer.inner.get(self.cursor), Some(b'\\')));
        let start_marker = self.cursor;
        let make_location = || self.buffer.calculate_location(start_marker);
        self.bump();
        match self.peek() {
            Some(b'\n') => {
                self.bump();
                self.next()
            }
            Some(space) if space.is_ascii_whitespace() => {
                self.bump();
                self.eat_whitespace();
                match self.peek() {
                    Some(b'\n') => {
                        self.bump();
                        self.next()
                    }
                    Some(_) => Some(Err(Error::new_parse(
                        SyntaxError::InvalidEscape,
                        make_location(),
                    ))),
                    None => None,
                }
            }
            Some(_) => {
                self.rewind(start_marker);
                Some(Ok(self.parse_ident()))
            }
            None => None,
        }
    }
}

impl Iterator for Tokenizer<'_> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.peek() {
            Some(b'\n') => {
                let start = self.cursor;
                self.bump();
                Some(Ok(Token::Newline(start)))
            }
            Some(space) if space.is_ascii_whitespace() => {
                self.bump();
                self.eat_whitespace();
                self.next()
            }
            Some(b'\\') => self.next_byte_escape(),
            Some(b'#') => {
                self.bump();
                while let Some(byte) = self.peek() {
                    if byte == b'\n' {
                        break;
                    }
                    self.bump();
                }
                self.next()
            }
            Some(_) => Some(Ok(self.parse_ident())),
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.buffer.inner.len() - self.cursor))
    }
}

impl BlifBuffer {
    fn tokenize(&self) -> Tokenizer {
        Tokenizer::new(self)
    }
}

pub struct BlifReader {
    buffer: BlifBuffer,
}

impl BlifReader {
    pub fn from_reader<R: Read>(mut reader: R, filename: Option<&str>) -> Result<Self> {
        let mut buffer = BlifBuffer::default();
        reader.read_to_end(&mut buffer.inner)?;
        buffer.filename = filename.map(str::to_string);
        Ok(Self { buffer })
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
    fn test_calculate_source_location() {
        let buffer = BlifBuffer::from(
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
        assert_eq!(buffer.inner.len(), 62);
        assert_eq!(buffer.calculate_location(60), loc!(6, 4));

        let buffer = BlifBuffer::from("\na");
        assert_eq!(buffer.calculate_location(0), loc!(1, 1));
        assert_eq!(buffer.calculate_location(1), loc!(2, 1));
    }

    macro_rules! tok {
        (newline @ $offset:expr) => {
            Token::Newline($offset)
        };
        (ident @ ( $offset:expr, $len:expr )) => {
            Token::Ident {
                offset: $offset,
                len: $len,
            }
        };
    }

    #[test]
    fn test_tokenizer() {
        let buffer = BlifBuffer::from(
            r#"a b c
1 2 \
34
"#,
        );
        let expected = [
            tok!(ident @ (0, 1)),
            tok!(ident @ (2, 1)),
            tok!(ident @ (4, 1)),
            tok!(newline @ 5),
            tok!(ident @ (6, 1)),
            tok!(ident @ (8, 1)),
            tok!(ident @ (12, 2)),
            tok!(newline @ 14),
        ];
        assert_eq!(
            buffer.tokenize().map(Result::unwrap).collect::<Vec<_>>(),
            &expected
        );
    }

    #[test]
    fn test_tokenize_comments() {
        let buffer = BlifBuffer::from(
            r#"# foo bar
a b # c d
# baz \
1 2
lorem ipsum
"#,
        );
        let expected = [
            tok!(newline @ 9),
            tok!(ident @ (10, 1)),
            tok!(ident @ (12, 1)),
            tok!(newline @ 19),
            tok!(newline @ 27),
            tok!(ident @ (28, 1)),
            tok!(ident @ (30, 1)),
            tok!(newline @ 31),
            tok!(ident @ (32, 5)),
            tok!(ident @ (38, 5)),
            tok!(newline @ 43),
        ];
        assert_eq!(
            buffer.tokenize().map(Result::unwrap).collect::<Vec<_>>(),
            &expected
        );
    }
}
