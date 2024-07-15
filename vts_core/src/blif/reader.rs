#![allow(dead_code)]

use std::fmt;
use std::io::Read;

use thiserror::Error;

use super::netlist::Netlist;

use crate::bytescanner::Scanner;

trait BlifChar {
    fn is_line_whitespace(&self) -> bool;
    fn is_directive_start(&self) -> bool;
    fn is_directive_continue(&self) -> bool;
}

impl BlifChar for u8 {
    /// Returns `true` if the byte is ascii whitespace (excluding newlines),
    /// else `false`.
    #[inline]
    fn is_line_whitespace(&self) -> bool {
        *self != b'\n' && self.is_ascii_whitespace()
    }

    /// Returns `true` if the byte is the start of a directive ('.').
    #[inline]
    fn is_directive_start(&self) -> bool {
        *self == b'.'
    }

    /// Returns `true` if the byte is a directive continue (alphabetic).
    #[inline]
    fn is_directive_continue(&self) -> bool {
        self.is_ascii_alphabetic()
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
                if self.0.bytes().any(|b| b.is_ascii_whitespace()) {
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

/// An owned buffer of BLIF text/bytes.
#[derive(Debug, Default)]
struct BlifBuffer {
    filename: Option<String>,
    inner: Box<[u8]>,
}

#[derive(Debug, PartialEq)]
struct EscapedNewline {
    escape_i: usize,
    newline_i: usize,
}

impl From<(usize, usize)> for EscapedNewline {
    fn from(indices: (usize, usize)) -> Self {
        Self {
            escape_i: indices.0,
            newline_i: indices.1,
        }
    }
}

#[derive(Debug)]
struct BlifLines {
    line_indices: Box<[usize]>,
    escaped_newline_indices: Box<[EscapedNewline]>,
}

impl BlifBuffer {
    /// Create a new buffer with an optional filename.
    fn new<I>(bytes: I, filename: Option<String>) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        Self {
            filename,
            inner: bytes.into_iter().collect(),
        }
    }

    /// Create a new buffer by copying a string.
    fn new_str(input: &str) -> Self {
        Self::new(input.bytes(), None)
    }

    /// Calculate the 1-based line number and column offset at `offset`.
    ///
    /// Panics if `offset` is out of bounds.
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

    /// Preprocess the buffer and replace newline escapes.
    ///
    /// Returns
    /// - a list of offsets to newline characters (excluding escaped
    ///   newlines).
    /// - a list of offsets to escaped newlines (along with the
    ///   offset to the associated escape character).
    ///
    /// as a single struct, [BlifLines].
    fn preprocess(&mut self) -> BlifLines {
        let mut line_indices = Vec::new();
        let mut escaped_newline_indices = Vec::new();
        let mut scanner = Scanner::new(&self.inner);
        while !scanner.done() {
            let start = scanner.cursor();
            let line = scanner.eat_until(b'\n');
            scanner.eat();
            // Check for a newline escape, i.e. "\\\n".
            // Also support arbitrary whitespace between the '\\' and '\n',
            // including carriage returns ('\r') used by older systems.
            let maybe_escape_i = line.len()
                - (1 + line
                    .iter()
                    .rev()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count());
            if line[maybe_escape_i] == b'\\' {
                let escape_i = start + maybe_escape_i;
                let newline_i = start + line.len();
                escaped_newline_indices.push((escape_i, newline_i));
            } else {
                line_indices.push(start);
            }
        }
        // Replace escaped newlines (and associated escape character) with
        // spaces to simplify the implementation of the tokenizer.
        // TODO(rikus): investigate performance of handling this in-line the
        // tokenizer.
        for (escape_i, newline_i) in escaped_newline_indices.iter() {
            self.inner[*escape_i] = b' ';
            self.inner[*newline_i] = b' ';
        }
        BlifLines {
            line_indices: Box::from_iter(line_indices),
            escaped_newline_indices: Box::from_iter(
                escaped_newline_indices
                    .into_iter()
                    .map(EscapedNewline::from),
            ),
        }
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

/// A spanned location in the buffer.
#[derive(Debug, PartialEq)]
struct Span {
    start: usize,
    len: usize,
}

impl Span {
    fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    fn new_range(start: usize, end: usize) -> Self {
        Self::new(start, end - start)
    }
}

/// A list of [Span]s.
#[derive(Debug, PartialEq)]
struct SpanList {
    spans: Box<[Span]>,
}

/// A multi-input, single-output PLA description.
#[derive(Debug, PartialEq)]
struct Cover {
    input: Span,
    output: Span,
    span: Span,
}

/// A `formal=actual` pair in a subcircuit instantiation.
#[derive(Debug, PartialEq)]
struct FormalActual {
    input: Span,
    output: Span,
}

/// A BLIF directive.
#[derive(Debug, PartialEq)]
enum Directive {
    /// `.model <name>`
    Model { name: Span, span: Span },
    /// `.inputs <name0> [<name1> ...]`
    Inputs { list: SpanList, span: Span },
    /// `.outputs <name0> [<name1> ...]`
    Outputs { list: SpanList, span: Span },
    /// `.names <input0> [<input1> ...] <output>`
    Names {
        input: SpanList,
        output: SpanList,
        span: Span,
    },
    /// `.latch <input> <output> [<ty> <ctrl>] [<init>]`
    Latch {
        input: Span,
        output: Span,
        ty: Option<Span>,
        ctrl: Option<Span>,
        init: Option<Span>,
        span: Span,
    },
    /// `.subckt <name> <formal0>=<actual0> [<formal1>=<actual1> ...]`
    Subckt {
        name: Span,
        formal_actual: Box<[FormalActual]>,
        span: Span,
    },
    /// `.end`
    ///
    /// NOTE: `End` directives can be implicit and simply mark the location
    /// where the model ends (in which case the offset is given in `span.start`
    /// and `span.len == 0`).
    End { span: Span },
}

impl Directive {
    /// The entire span of the directive.
    fn span(&self) -> &Span {
        match self {
            Self::Model { span, .. } => span,
            Self::Inputs { span, .. } => span,
            Self::Outputs { span, .. } => span,
            Self::Names { span, .. } => span,
            Self::Latch { span, .. } => span,
            Self::Subckt { span, .. } => span,
            Self::End { span } => span,
        }
    }
}

/// A token -- roughly corrosponds to a single line in BLIF text/bytes.
#[derive(Debug, PartialEq)]
enum Token {
    /// A [Directive] token.
    Directive(Directive),
    /// An unknown directive (for error reporting).
    UnknownDirective(Span),
    /// A [Cover] token.
    Cover(Cover),
    /// An implicit token yielded after the PLA description of a [Directive]
    /// directive. `span.start` is the end of the final [Cover] token.
    NamesEnd(Span),
}

impl Token {
    /// The entire span of the token.
    fn span(&self) -> &Span {
        match self {
            Self::Directive(directive) => directive.span(),
            Self::UnknownDirective(span) => span,
            Self::Cover(cover) => &cover.span,
            Self::NamesEnd(span) => span,
        }
    }
}

/// An iterator over scanned tokens.
struct Tokenizer<'a> {
    scanner: Scanner<'a>,
    lines: BlifLines,
    current_line: usize,
}

impl<'a> Tokenizer<'a> {
    /// Start a new tokenizer.
    fn new(buffer: &'a BlifBuffer, lines: BlifLines) -> Self {
        Self {
            scanner: Scanner::new(&buffer.inner),
            lines,
            current_line: 0,
        }
    }

    /// Preprocess `buffer` and start a new tokenizer.
    fn new_preprocess(buffer: &'a mut BlifBuffer) -> Self {
        let lines = buffer.preprocess();
        Self::new(buffer, lines)
    }

    /// The offset to the start of the current line in the buffer.
    fn current_line_offset(&self) -> usize {
        self.lines.line_indices[self.current_line]
    }
}

impl Iterator for Tokenizer<'_> {
    type Item = Token;

    /// Get the next token in the buffer skipping whitespace, newlines and
    /// comments.
    fn next(&mut self) -> Option<Self::Item> {
        let mut token = None;
        while !self.scanner.done() && token.is_none() {
            // Skip whitespace, excluding newlines.
            self.scanner.eat_while(BlifChar::is_line_whitespace);
            // Skip comments.
            if self.scanner.eat_if(b'#') {
                self.scanner.eat_until(b'\n');
                self.scanner.eat();
                self.current_line += 1;
                continue;
            }
            // Skip newlines.
            if self.scanner.eat_if(b'\n') {
                self.current_line += 1;
                continue;
            }
            let start = self.scanner.cursor();
            // Tokenize directives.
            if self.scanner.eat_if(BlifChar::is_directive_start) {
                let directive = self.scanner.eat_while(BlifChar::is_directive_continue);
                match directive {
                    b"model" => {
                        // TODO(rikus): handle model
                        todo!()
                    }
                    b"inputs" => {
                        // TODO(rikus): handle inputs
                        todo!()
                    }
                    b"outputs" => {
                        // TODO(rikus): handle outputs
                        todo!()
                    }
                    b"names" => {
                        // TODO(rikus): handle names
                        todo!()
                    }
                    b"latch" => {
                        // TODO(rikus): handle latch
                        todo!()
                    }
                    b"subckt" => {
                        // TODO(rikus): handle subckt
                        todo!()
                    }
                    b"end" => {
                        // TODO(rikus): handle end
                        todo!()
                    }
                    _unknown => {
                        // TODO(rikus): look out for directives that we don't
                        // support and report those as separate errors.
                        token = Some(Token::UnknownDirective(Span::new_range(
                            start,
                            self.scanner.cursor(),
                        )));
                    }
                }
                break;
            }
            // TODO(rikus): tokenize other cases and report unexpected bytes.
            let eof = self.scanner.bytes().len();
            self.scanner.jump(eof);
            todo!()
        }
        token
    }
}

impl BlifBuffer {
    /// Preprocess and create a new [Tokenizer] from the buffer.
    fn tokenize(&mut self) -> Tokenizer {
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
        assert_eq!(buffer.inner.len(), 62);
        assert_eq!(buffer.calculate_location(60), loc!(6, 4));

        let buffer = BlifBuffer::new_str("\na");
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
