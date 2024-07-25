use std::io::Read;
use std::str::from_utf8;

use super::buffer::{BlifBuffer, Span};
use super::command::Command;
use super::error::{Filename, Result};
use super::netlist::Netlist;
use super::token::{Token, TokenKind};

pub trait BlifEvents<'a, 'tok> {
    fn model(&mut self, name: Option<&'a str>, token: &'tok Token) -> Result<()> {
        let _name = name;
        let _token = token;
        Ok(())
    }

    fn inputs(&mut self, inputs: &'_ [&'a str], token: &'tok Token) -> Result<()> {
        let _inputs = inputs;
        let _token = token;
        Ok(())
    }

    fn outputs(&mut self, outputs: &'_ [&'a str], token: &'tok Token) -> Result<()> {
        let _outputs = outputs;
        let _token = token;
        Ok(())
    }

    fn names(&mut self, inputs: &'_ [&'a str], output: &'a str, token: &'tok Token) -> Result<()> {
        let _inputs = inputs;
        let _output = output;
        let _token = token;
        Ok(())
    }

    fn latch(
        &mut self,
        input: &'a str,
        output: &'a str,
        trigger: Option<&'a str>,
        ctrl: Option<&'a str>,
        init: Option<&'a str>,
        token: &'tok Token,
    ) -> Result<()> {
        let _input = input;
        let _output = output;
        let _trigger = trigger;
        let _ctrl = ctrl;
        let _init = init;
        let _token = token;
        Ok(())
    }

    fn subckt(&mut self, token: &'tok Token) -> Result<()> {
        let _token = token;
        Ok(())
    }

    fn end(&mut self, token: &'tok Token) -> Result<()> {
        let _token = token;
        Ok(())
    }

    fn cube(&mut self, token: &'tok Token) -> Result<()> {
        let _token = token;
        Ok(())
    }
}

// TODO(rikus): Remove once we have a real implementor.
impl BlifEvents<'_, '_> for () {}

/// A BLIF file reader.
#[derive(Debug)]
pub struct BlifReader {
    buffer: BlifBuffer,
}

impl BlifReader {
    fn parse_model<'a, 'tok, E>(
        &'a self,
        name: Option<Span>,
        token: &'tok Token,
        events: &mut E,
    ) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        // TODO(rikus): Report UTF-8 error.
        let name = name.map(|name| {
            from_utf8(self.buffer.view(name)).expect("model name should be valid utf-8")
        });
        events.model(name, token)?;
        Ok(())
    }

    fn parse_inputs<'a, 'tok, E>(
        &'a self,
        inputs: &'_ [Span],
        token: &'tok Token,
        events: &mut E,
    ) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        // TODO(rikus): Report UTF-8 error.
        let inputs = inputs
            .iter()
            .map(|&input| {
                from_utf8(self.buffer.view(input)).expect("input name should be valid utf-8")
            })
            .collect::<Vec<_>>();
        events.inputs(&inputs, token)?;
        Ok(())
    }

    fn parse_outputs<'a, 'tok, E>(
        &'a self,
        outputs: &'_ [Span],
        token: &'tok Token,
        events: &mut E,
    ) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        // TODO(rikus): Report UTF-8 error.
        let outputs = outputs
            .iter()
            .map(|&output| {
                from_utf8(self.buffer.view(output)).expect("output name should be valid utf-8")
            })
            .collect::<Vec<_>>();
        events.outputs(&outputs, token)?;
        Ok(())
    }

    fn parse_names<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        Ok(())
    }

    fn parse_latch<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        Ok(())
    }

    fn parse_subckt<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        Ok(())
    }

    fn parse_end<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        Ok(())
    }

    fn parse_command<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        debug_assert_eq!(token.kind, TokenKind::Command);
        match Command::parse_trivia(token.trivia.iter().copied(), &self.buffer)? {
            Command::Model { name, .. } => {
                self.parse_model(name, token, events)?;
            }
            Command::Inputs { inputs, .. } => {
                self.parse_inputs(&inputs, token, events)?;
            }
            Command::Outputs { outputs, .. } => {
                self.parse_outputs(&outputs, token, events)?;
            }
            Command::Names { .. } => {
                self.parse_names(token, events)?;
            }
            Command::Latch { .. } => {
                self.parse_latch(token, events)?;
            }
            Command::Subckt { .. } => {
                self.parse_subckt(token, events)?;
            }
            Command::End { .. } => {
                self.parse_end(token, events)?;
            }
        }
        Ok(())
    }

    fn parse_cube<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        debug_assert_eq!(token.kind, TokenKind::Cube);
        Ok(())
    }

    fn parse_token<'a, 'tok, E>(&'a self, token: &'tok Token, events: &mut E) -> Result<()>
    where
        E: BlifEvents<'a, 'tok>,
    {
        match token.kind {
            TokenKind::Command => {
                self.parse_command(token, events)?;
            }
            TokenKind::Cube => {
                self.parse_cube(token, events)?;
            }
        }
        Ok(())
    }

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
        for token in self.buffer.tokenize() {
            let token = token?;
            self.parse_token(&token, &mut ())?;
        }
        todo!()
    }
}

#[cfg(test)]
mod tests {}
