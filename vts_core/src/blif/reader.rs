use std::io::Read;

use super::buffer::BlifBuffer;
use super::error::Result;
use super::netlist::Netlist;

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
        buffer.filename = filename.map(str::to_string);
        Self { buffer }
    }

    pub fn parse_netlist(&mut self) -> Result<Netlist> {
        let tokenizer = self.buffer.tokenize();
        let _ = tokenizer.count();
        todo!()
    }
}

#[cfg(test)]
mod tests {}
