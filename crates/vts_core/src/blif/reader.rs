use std::io::Read;

use super::buffer::BlifBuffer;
use super::command::Command;
use super::error::{Filename, Result};
use super::netlist::Netlist;
use super::token::TokenKind;

/// A BLIF file reader.
#[derive(Debug)]
pub struct BlifReader {
    buffer: BlifBuffer,
}

impl BlifReader {
    pub fn from_reader<R: Read>(reader: R, filename: Option<&str>) -> Result<Self> {
        Ok(Self {
            buffer: BlifBuffer::from_reader(reader, filename)?,
        })
    }

    pub fn from_str(input: &str, filename: Option<&str>) -> Self {
        let mut buffer = BlifBuffer::new_str(input);
        buffer.filename = Filename::from(filename.map(str::to_string));
        Self { buffer }
    }

    pub fn parse_netlist(&mut self) -> Result<Netlist> {
        let mut tokenizer = self.buffer.tokenize();
        while let Some(token) = tokenizer.next().transpose()? {
            match token.kind {
                TokenKind::Command => {
                    let _command =
                        Command::parse_trivia(token.trivia.iter().copied(), &self.buffer)?;
                    todo!();
                }
                TokenKind::Cube => {
                    // TODO(rikus): Parse cubes.
                    todo!();
                }
            }
        }
        todo!()
    }
}

#[cfg(test)]
mod tests {}
