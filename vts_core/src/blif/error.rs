use std::fmt;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] TaggedParseError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("an unexpected error occurred: {0}")]
    Unknown(String),
}

#[derive(Clone, Debug, Error)]
pub enum ParseError {
    #[error("an unexpected parsing error occurred: {0}")]
    Unknown(String),
}

#[derive(Clone, Debug, Error)]
#[error(
    r#"{error}
    
while parsing {location}"#
)]
pub struct TaggedParseError {
    error: ParseError,
    location: SourceLocation,
}

// TODO(rikus): Should this be moved out of `blif` and to own submodule?
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Filename {
    /// A generic filename.
    File(String),
    /// Standard input.
    Stdin,
    /// An unknown source.
    #[default]
    Unknown,
    /// A test.
    #[cfg(test)]
    Test,
}

impl From<String> for Filename {
    fn from(filename: String) -> Self {
        match filename.as_str() {
            "-" | "<stdin>" => Filename::Stdin,
            "<unknown>" => Filename::Unknown,
            #[cfg(test)]
            "<test>" => Filename::Test,
            _ => Self::File(filename),
        }
    }
}

impl From<Option<String>> for Filename {
    fn from(filename: Option<String>) -> Self {
        if let Some(filename) = filename {
            Self::from(filename)
        } else {
            Self::Unknown
        }
    }
}

impl fmt::Display for Filename {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(filename) => {
                if filename.chars().any(char::is_whitespace) {
                    write!(formatter, "\"{}\"", filename)
                } else {
                    formatter.write_str(filename)
                }
            }
            Self::Stdin => formatter.write_str("<stdin>"),
            Self::Unknown => formatter.write_str("<unknown>"),
            #[cfg(test)]
            Self::Test => formatter.write_str("<test>"),
        }
    }
}

/// A location in BLIF text/bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    /// The source file name.
    pub filename: Filename,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column offset.
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}:{}", self.filename, self.line, self.column)
    }
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

pub(super) trait ParseLocation<T> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blif::buffer::{BlifBuffer, BytePos};

    macro_rules! check_loc {
        ($buffer:expr, $pos:expr => ($line:expr, $col:expr)) => {
            assert_eq!(
                $buffer.calculate_location(BytePos($pos)),
                SourceLocation {
                    line: $line,
                    column: $col,
                    filename: Filename::Test,
                }
            );
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
        check_loc!(buffer, 0 => (1, 1));
        check_loc!(buffer, 9 => (1, 10));
        check_loc!(buffer, 10 => (1, 11));
        check_loc!(buffer, 11 => (2, 1));
        check_loc!(buffer, 25 => (3, 1));
        assert_eq!(buffer.len(), 62);
        check_loc!(buffer, 60 => (6, 4));

        let buffer = BlifBuffer::new_str("\na");
        check_loc!(buffer, 0 => (1, 1));
        check_loc!(buffer, 1 => (2, 1));
    }
}
