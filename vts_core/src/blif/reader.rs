use std::fmt;

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.file.bytes().any(|b| b.is_ascii_whitespace()) {
            write!(formatter, "\"{}\":{}:{}", self.file, self.line, self.column)
        } else {
            write!(formatter, "{}:{}:{}", self.file, self.line, self.column)
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseError {
    #[error(r#"unknown directive "{0}""#)]
    UnknownDirective(String),
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

struct BlifBuffer {
    inner: Vec<u8>,
}

impl From<String> for BlifBuffer {
    fn from(input: String) -> Self {
        Self {
            inner: input.into_bytes(),
        }
    }
}

struct Tokenizer<'a> {
    buffer: &'a BlifBuffer,
    cursor: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(buffer: &'a BlifBuffer) -> Self {
        Self { buffer, cursor: 0 }
    }

    fn bump(&mut self) {
        if self.cursor + 1 < self.buffer.inner.len() {
            self.cursor += 1;
        }
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
    pub fn new() -> Self {
        todo!()
    }
}
