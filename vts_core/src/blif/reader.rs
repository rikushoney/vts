use std::fmt;
use std::io::Read;

use thiserror::Error;

use super::netlist::Netlist;

use crate::bytescanner::Scanner;

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

#[derive(Debug, PartialEq)]
struct Span {
    start: usize,
    len: usize,
}

#[derive(Debug, PartialEq)]
struct Cover {
    input: Span,
    output: Span,
}

#[derive(Debug, PartialEq)]
struct FormalActual {
    input: Span,
    output: Span,
}

#[derive(Debug, PartialEq)]
enum Directive {
    Model {
        name: Span,
        span: Span,
    },
    Inputs {
        list: Vec<Span>,
        span: Span,
    },
    Outputs {
        list: Vec<Span>,
        span: Span,
    },
    Names {
        inputs: Vec<Span>,
        output: Vec<Span>,
        covers: Vec<Cover>,
        span: Span,
    },
    Latch {
        input: Span,
        output: Span,
        ty: Option<Span>,
        control: Option<Span>,
        init: Option<Span>,
        span: Span,
    },
    Subckt {
        name: Span,
        formal_actual: Vec<FormalActual>,
        span: Span,
    },
}

impl Directive {
    fn span(&self) -> &Span {
        match self {
            Self::Model { span, .. } => span,
            Self::Inputs { span, .. } => span,
            Self::Outputs { span, .. } => span,
            Self::Names { span, .. } => span,
            Self::Latch { span, .. } => span,
            Self::Subckt { span, .. } => span,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Directive(Directive),
    Whitespace(Span),
    Newline(Span),
    Comment(Span),
}

impl Token {
    fn span(&self) -> &Span {
        match self {
            Self::Directive(directive) => directive.span(),
            Self::Whitespace(span) => span,
            Self::Newline(span) => span,
            Self::Comment(span) => span,
        }
    }
}

struct Tokenizer<'a> {
    scanner: Scanner<'a>,
}

impl<'a> Tokenizer<'a> {
    fn new(buffer: &'a BlifBuffer) -> Self {
        Self {
            scanner: Scanner::new(&buffer.inner),
        }
    }
}

impl Iterator for Tokenizer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.scanner.cursor();
        if self.scanner.eat_if(u8::is_ascii_whitespace) {
            self.scanner.eat_whitespace();
            return Some(Token::Whitespace(Span {
                start,
                len: self.scanner.cursor() - start,
            }));
        }
        if self.scanner.eat_if(b'#') {
            self.scanner.eat_until(b'\n');
            return Some(Token::Comment(Span {
                start,
                len: self.scanner.cursor() - start,
            }));
        }
        todo!()
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
        // let _ = tokenizer.count();
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

    //     macro_rules! tok {
    //         (newline @ $offset:expr) => {
    //             Token::Newline($offset)
    //         };
    //         (ident @ ( $offset:expr, $len:expr )) => {
    //             Token::Ident {
    //                 offset: $offset,
    //                 len: $len,
    //             }
    //         };
    //     }

    //     #[test]
    //     fn test_tokenizer() {
    //         let buffer = BlifBuffer::from(
    //             r#"a b c
    // 1 2 \
    // 34
    // "#,
    //         );
    //         let expected = [
    //             tok!(ident @ (0, 1)),
    //             tok!(ident @ (2, 1)),
    //             tok!(ident @ (4, 1)),
    //             tok!(newline @ 5),
    //             tok!(ident @ (6, 1)),
    //             tok!(ident @ (8, 1)),
    //             tok!(ident @ (12, 2)),
    //             tok!(newline @ 14),
    //         ];
    //         assert_eq!(
    //             buffer.tokenize().map(Result::unwrap).collect::<Vec<_>>(),
    //             &expected
    //         );
    //     }

    //     #[test]
    //     fn test_tokenize_comments() {
    //         let buffer = BlifBuffer::from(
    //             r#"# foo bar
    // a b # c d
    // # baz \
    // 1 2
    // lorem ipsum
    // "#,
    //         );
    //         let expected = [
    //             tok!(newline @ 9),
    //             tok!(ident @ (10, 1)),
    //             tok!(ident @ (12, 1)),
    //             tok!(newline @ 19),
    //             tok!(newline @ 27),
    //             tok!(ident @ (28, 1)),
    //             tok!(ident @ (30, 1)),
    //             tok!(newline @ 31),
    //             tok!(ident @ (32, 5)),
    //             tok!(ident @ (38, 5)),
    //             tok!(newline @ 43),
    //         ];
    //         assert_eq!(
    //             buffer.tokenize().map(Result::unwrap).collect::<Vec<_>>(),
    //             &expected
    //         );
    //     }
}
