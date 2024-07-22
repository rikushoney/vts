use thiserror::Error;

use super::buffer::{BlifBuffer, Span};
use super::error::SourceLocation;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(r#"unknown command "{0}""#)]
    UnknownCommand(String),
}

#[derive(Debug, Error)]
#[error(
    r#"{error}
    
while parsing {location}"#
)]
pub struct TaggedParseError {
    error: ParseError,
    location: SourceLocation,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

// TODO(rikus): Merge with blif crate error.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parse(TaggedParseError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A BLIF command.
pub enum Command {
    Model { span: Span },
    Inputs { span: Span },
    Outputs { span: Span },
    Names { span: Span },
    Latch { span: Span },
    Subckt { span: Span },
    End { span: Span },
}

impl Command {
    /// Try to parse trivia as a command.
    ///
    /// Returns `Ok(Command)` on success or `Err(Error)` on failure.
    /// Panics if `trivia` does not yield at least a single token or if the
    /// first token does not start with a `.`.
    pub fn parse_trivia<I>(mut trivia: I, buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let name_extent = trivia
            .next()
            .expect("trivia should yield at least a single token");
        let name = buffer.view(name_extent);
        assert!(name.starts_with(b"."));
        match &name[1..] {
            b"model" => {
                // TODO: Parse model line.
                todo!()
            }
            b"inputs" => {
                // TODO: Parse inputs line.
                todo!()
            }
            b"outputs" => {
                // TODO: Parse outputs line.
                todo!()
            }
            b"names" => {
                // TODO: Parse names line.
                todo!()
            }
            b"latch" => {
                // TODO: Parse latch line.
                todo!()
            }
            b"subckt" => {
                // TODO: Parse subckt line.
                todo!()
            }
            b"end" => {
                // TODO: Parse end line.
                todo!()
            }
            unknown => {
                // TODO: Report unknown commands and potentially known but
                // unsupported commands.
                panic!("unknown command {:?}", unknown);
            }
        }
    }

    /// The full extent of the command.
    pub fn span(&self) -> &Span {
        match self {
            Self::Model { span } => span,
            Self::Inputs { span } => span,
            Self::Outputs { span } => span,
            Self::Names { span } => span,
            Self::Latch { span } => span,
            Self::Subckt { span } => span,
            Self::End { span } => span,
        }
    }
}
