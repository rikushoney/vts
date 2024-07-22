//! AIGER file format reader and writer
//!
//! Reference: <https://fmv.jku.at/aiger/FORMAT-20070427.pdf>

use itertools::Itertools;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Read(String),
    #[error("{0}")]
    Write(String),
}

#[derive(Debug, PartialEq)]
pub enum AigerFormat {
    Ascii,
    Bin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Literal(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Variable(usize);

#[cfg(test)]
#[derive(PartialEq)]
enum Negated {
    True,
    False,
}

impl Literal {
    #[cfg(test)]
    fn new_variable(var: Variable, negated: Negated) -> Self {
        assert!(var.0 > 0);
        let lsb = match negated {
            Negated::True => 1,
            Negated::False => 0,
        };
        Self((var.0 << 1) | lsb)
    }

    #[cfg(test)]
    fn new_const(c: bool) -> Self {
        if c {
            Self(1)
        } else {
            Self(0)
        }
    }

    fn to_latch(self, next: Literal) -> Latch {
        Latch(self.to_variable(), next)
    }

    fn to_gate(self, param_1: Literal, param_2: Literal) -> Gate {
        Gate(self.to_variable(), param_1, param_2)
    }

    fn to_variable(self) -> Variable {
        Variable(self.0 >> 1)
    }

    fn is_const(&self) -> bool {
        self.0 < 2
    }

    fn is_negated(&self) -> bool {
        self.0 & 1 == 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Latch(Variable, Literal);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Gate(Variable, Literal, Literal);

#[derive(Default, Debug, PartialEq)]
pub struct Aig {
    inputs: Box<[Variable]>,
    latches: Box<[Latch]>,
    outputs: Box<[Literal]>,
    gates: Box<[Gate]>,
}

impl FromStr for Aig {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut reader = AigReader::new(text);
        reader.read_all()
    }
}

#[derive(Default, Debug, PartialEq)]
struct AigHeader {
    format: Option<AigerFormat>,
    max_idx: usize,
    n_inputs: usize,
    n_latches: usize,
    n_outputs: usize,
    n_gates: usize,
}

struct AigReader<'a> {
    header: AigHeader,
    body: Aig,
    source: &'a [u8],
    cursor: usize,
}

impl<'a> Iterator for AigReader<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.source.len() {
            let b = self.source[self.cursor];
            self.cursor += 1;
            Some(b)
        } else {
            None
        }
    }
}

macro_rules! s {
    ($text:literal) => {
        $text.to_string()
    };
    ($bytes:expr) => {
        std::str::from_utf8($bytes).expect("should be valid utf-8")
    };
    ($($byte:expr),* $(,)?) => {
        s!(&[$($byte),*])
    }
}

const RADIX: usize = 10;

impl<'a> AigReader<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            header: AigHeader::default(),
            body: Aig::default(),
            source: text.as_bytes(),
            cursor: 0,
        }
    }

    fn reset(&mut self) -> Aig {
        self.cursor = 0;
        self.header = AigHeader::default();
        std::mem::take(&mut self.body)
    }

    fn read_magic(&mut self) -> Result<AigerFormat, Error> {
        assert_eq!(self.cursor, 0);
        if let Some(magic) = self.next_tuple() {
            match magic {
                (b'a', b'a', b'g') => Ok(AigerFormat::Ascii),
                (b'a', b'i', b'g') => Ok(AigerFormat::Bin),
                _ => Err(Error::Read(format!(
                    r#"unknown magic "{}""#,
                    s!(magic.0, magic.1, magic.2)
                ))),
            }
        } else {
            Err(Error::Read(format!(
                r#"expected "aag" or "aig", not "{}""#,
                s!(self.source)
            )))
        }
    }

    fn eof(&self) -> bool {
        self.cursor == self.source.len()
    }

    fn iter(&self) -> impl Iterator<Item = &u8> {
        self.source.iter().skip(self.cursor)
    }

    fn next_or_empty(&mut self) -> String {
        self.next().map(|b| s!(b,).to_string()).unwrap_or(s!(""))
    }

    fn read_usize(&mut self) -> Result<usize, Error> {
        let start = self.cursor;
        let count = self.iter().take_while(|&b| b.is_ascii_digit()).count();
        let end = start + count;
        if start == end {
            let invalid = self.next_or_empty();
            return Err(Error::Read(format!(
                "expected digit in [0-9], not '{invalid}'",
            )));
        }
        self.cursor += count;
        let digits = &self.source[start..end];
        let mut number: usize = 0;
        for digit in digits {
            if let Some(updated) = number
                .checked_mul(RADIX)
                .and_then(|number| number.checked_add((digit - b'0') as usize))
            {
                number = updated;
            } else {
                let digits = s!(digits);
                let max = usize::MAX;
                return Err(Error::Read(format!("integer overflow ({digits} > {max})",)));
            }
        }
        Ok(number)
    }

    fn consume(&mut self, target: u8) -> Result<u8, Error> {
        let cur = self.cursor;
        match self.next() {
            Some(actual) if target == actual => Ok(actual),
            Some(actual) => {
                self.cursor = cur;
                let target = target as char;
                let actual = actual as char;
                Err(Error::Read(format!("expected '{target}', not '{actual}'")))
            }
            None => {
                let target = target as char;
                Err(Error::Read(format!("expected '{target}'")))
            }
        }
    }

    fn maybe_consume(&mut self, target: u8) {
        let _ = self.consume(target);
    }

    fn consume_space(&mut self) -> Result<u8, Error> {
        self.consume(b' ')
    }

    fn consume_newline(&mut self) -> Result<u8, Error> {
        self.maybe_consume(b'\r');
        self.consume(b'\n')
    }

    fn read_header(&mut self) -> Result<AigHeader, Error> {
        let magic = self.read_magic()?;
        self.consume_space()?;
        let max_idx = self.read_usize()?;
        self.consume_space()?;
        let n_inputs = self.read_usize()?;
        self.consume_space()?;
        let n_latches = self.read_usize()?;
        self.consume_space()?;
        let n_outputs = self.read_usize()?;
        self.consume_space()?;
        let n_gates = self.read_usize()?;
        Ok(AigHeader {
            format: Some(magic),
            max_idx,
            n_inputs,
            n_latches,
            n_outputs,
            n_gates,
        })
    }

    fn read_inputs(&mut self) -> Result<Vec<Variable>, Error> {
        let mut inputs = Vec::with_capacity(self.header.n_inputs);
        for _ in 0..self.header.n_inputs {
            let lit = Literal(self.read_usize()?);
            if lit.is_const() {
                return Err(Error::Read(s!("constant input is invalid")));
            }
            if lit.is_negated() {
                return Err(Error::Read(s!("negated input is invalid")));
            }
            inputs.push(lit.to_variable());
            if self.consume_newline().is_err() {
                break;
            }
        }
        Ok(inputs)
    }

    fn read_latches(&mut self) -> Result<Vec<Latch>, Error> {
        let mut latches = Vec::with_capacity(self.header.n_latches);
        for _ in 0..self.header.n_latches {
            let lit = Literal(self.read_usize()?);
            if lit.is_const() {
                return Err(Error::Read(s!("constant latch is invalid")));
            }
            if lit.is_negated() {
                return Err(Error::Read(s!("negated latch is invalid")));
            }
            self.consume_space()?;
            let next = Literal(self.read_usize()?);
            latches.push(lit.to_latch(next));
            if self.consume_newline().is_err() {
                break;
            }
        }
        Ok(latches)
    }

    fn read_outputs(&mut self) -> Result<Vec<Literal>, Error> {
        let mut outputs = Vec::with_capacity(self.header.n_outputs);
        for _ in 0..self.header.n_outputs {
            let lit = Literal(self.read_usize()?);
            outputs.push(lit);
            if self.consume_newline().is_err() {
                break;
            }
        }
        Ok(outputs)
    }

    fn read_gates(&mut self) -> Result<Vec<Gate>, Error> {
        let mut gates = Vec::with_capacity(self.header.n_gates);
        for _ in 0..self.header.n_gates {
            let lhs = Literal(self.read_usize()?);
            if lhs.is_const() {
                return Err(Error::Read(s!("constant gate is invalid")));
            }
            if lhs.is_negated() {
                return Err(Error::Read(s!("negated gate is invalid")));
            }
            self.consume_space()?;
            let rhs1 = Literal(self.read_usize()?);
            self.consume_space()?;
            let rhs2 = Literal(self.read_usize()?);
            gates.push(lhs.to_gate(rhs1, rhs2));
            if self.consume_newline().is_err() {
                break;
            }
        }
        Ok(gates)
    }

    fn read_body(&mut self) -> Result<(), Error> {
        let inputs = self.read_inputs()?;
        if inputs.len() < self.header.n_inputs {
            let target = self.header.n_inputs;
            let actual = inputs.len();
            return Err(Error::Read(format!(
                "expected {target} inputs (found {actual})"
            )));
        }
        self.body.inputs = Box::from(inputs);
        let latches = self.read_latches()?;
        if latches.len() < self.header.n_latches {
            let target = self.header.n_latches;
            let actual = latches.len();
            return Err(Error::Read(format!(
                "expected {target} latches (found {actual})"
            )));
        }
        self.body.latches = Box::from(latches);
        let outputs = self.read_outputs()?;
        if outputs.len() < self.header.n_outputs {
            let target = self.header.n_outputs;
            let actual = outputs.len();
            return Err(Error::Read(format!(
                "expected {target} outputs (found {actual})"
            )));
        }
        self.body.outputs = Box::from(outputs);
        let gates = self.read_gates()?;
        if gates.len() < self.header.n_gates {
            let target = self.header.n_gates;
            let actual = gates.len();
            return Err(Error::Read(format!(
                "expected {target} gates (found {actual})"
            )));
        }
        self.body.gates = Box::from(gates);
        Ok(())
    }

    fn ensure_well_formed(&self) -> Result<(), Error> {
        // TODO: check
        // eventually we want to migrate all checking to this function
        Ok(())
    }

    fn read_all(&mut self) -> Result<Aig, Error> {
        self.header = self.read_header()?;
        if matches!(self.header.format, Some(AigerFormat::Bin)) {
            return Err(Error::Read(s!("binary format not yet supported")));
        }
        if self.consume_newline().is_ok() && !self.eof() {
            self.read_body()?;
            let _ = self.consume_newline();
        } else if !self.eof() {
            return Err(Error::Read(s!("expected newline followed by body")));
        }
        assert!(self.eof());
        if matches!(self.header.format, Some(AigerFormat::Ascii)) {
            self.ensure_well_formed()?;
        }
        Ok(self.reset())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(Aig::from_str("aag 0 0 0 0 0").unwrap(), Aig::default());
        assert_eq!(Aig::from_str("aag 0 0 0 0 0\n").unwrap(), Aig::default());
    }

    #[test]
    fn test_read_usize() {
        assert_eq!(AigReader::new("42").read_usize().unwrap(), 42);
        let usize_max = format!("{}", usize::MAX);
        assert_eq!(AigReader::new(&usize_max).read_usize().unwrap(), usize::MAX);
        let mut usize_max_plus_one = usize_max.clone();
        let ones = usize_max_plus_one
            .pop()
            .unwrap()
            .to_digit(RADIX as u32)
            .unwrap();
        assert!(ones < 9);
        usize_max_plus_one.push(char::from_digit(ones + 1, RADIX as u32).unwrap());
        assert!(AigReader::new(&usize_max_plus_one).read_usize().is_err());
    }

    #[test]
    fn test_read_const() {
        let aig_true = Aig::from_str("aag 0 0 0 1 0\n1").unwrap();
        assert_eq!(aig_true.inputs.len(), 0);
        assert_eq!(aig_true.latches.len(), 0);
        assert_eq!(aig_true.outputs[0], Literal::new_const(true));
        let aig_false = Aig::from_str("aag 0 0 0 1 0\n0").unwrap();
        assert_eq!(aig_false.inputs.len(), 0);
        assert_eq!(aig_false.latches.len(), 0);
        assert_eq!(aig_false.outputs[0], Literal::new_const(false));
    }

    #[test]
    fn test_read_buffer() {
        let aig_buffer = Aig::from_str("aag 1 1 0 1 0\n2\n2").unwrap();
        assert_eq!(aig_buffer.inputs[0], Variable(1));
        assert_eq!(
            aig_buffer.outputs[0],
            Literal::new_variable(Variable(1), Negated::False)
        );
        let aig_inverter = Aig::from_str("aag 1 1 0 1 0\n2\n3").unwrap();
        assert_eq!(aig_inverter.inputs[0], Variable(1));
        assert_eq!(
            aig_inverter.outputs[0],
            Literal::new_variable(Variable(1), Negated::True)
        );
    }

    #[test]
    fn test_read_gate() {
        let aig_and = Aig::from_str("aag 3 2 0 1 1\n2\n4\n6\n6 2 4").unwrap();
        let input_0 = Variable(1);
        let input_1 = Variable(2);
        let output = Variable(3);
        assert_eq!(aig_and.inputs[0], input_0);
        assert_eq!(aig_and.inputs[1], input_1);
        assert_eq!(
            aig_and.outputs[0],
            Literal::new_variable(output, Negated::False)
        );
        assert_eq!(
            aig_and.gates[0],
            Gate(
                output,
                Literal::new_variable(input_0, Negated::False),
                Literal::new_variable(input_1, Negated::False)
            )
        );
        let aig_or = Aig::from_str("aag 3 2 0 1 1\n2\n4\n7\n6 3 5").unwrap();
        let input_0 = Variable(1);
        let input_1 = Variable(2);
        let output = Variable(3);
        assert_eq!(aig_or.inputs[0], input_0);
        assert_eq!(aig_or.inputs[1], input_1);
        assert_eq!(
            aig_or.outputs[0],
            Literal::new_variable(output, Negated::True)
        );
        assert_eq!(
            aig_or.gates[0],
            Gate(
                output,
                Literal::new_variable(input_0, Negated::True),
                Literal::new_variable(input_1, Negated::True)
            )
        );
    }

    #[test]
    fn test_read_adder() {
        let aig_adder =
            Aig::from_str("aag 7 2 0 2 3\n2\n4\n6\n12\n6 13 15\n12 2 4\n14 3 5").unwrap();
        let input_0 = Variable(1);
        let input_1 = Variable(2);
        let output_0 = Variable(3);
        let output_1 = Variable(6);
        let temp = Variable(7);
        assert_eq!(aig_adder.inputs[0], input_0);
        assert_eq!(aig_adder.inputs[1], input_1);
        assert_eq!(
            aig_adder.outputs[0],
            Literal::new_variable(output_0, Negated::False)
        );
        assert_eq!(
            aig_adder.outputs[1],
            Literal::new_variable(output_1, Negated::False)
        );
        assert_eq!(
            aig_adder.gates[0],
            Gate(
                output_0,
                Literal::new_variable(output_1, Negated::True),
                Literal::new_variable(temp, Negated::True)
            )
        );
        assert_eq!(
            aig_adder.gates[1],
            Gate(
                output_1,
                Literal::new_variable(input_0, Negated::False),
                Literal::new_variable(input_1, Negated::False),
            )
        );
        assert_eq!(
            aig_adder.gates[2],
            Gate(
                temp,
                Literal::new_variable(input_0, Negated::True),
                Literal::new_variable(input_1, Negated::True),
            )
        );
    }

    #[test]
    fn test_read_latch() {
        let aig_latch = Aig::from_str("aag 1 0 1 2 0\n2 3\n2\n3").unwrap();
        let output = Variable(1);
        assert_eq!(
            aig_latch.latches[0],
            Latch(output, Literal::new_variable(output, Negated::True))
        );
        assert_eq!(
            aig_latch.outputs[0],
            Literal::new_variable(output, Negated::False)
        );
        assert_eq!(
            aig_latch.outputs[1],
            Literal::new_variable(output, Negated::True)
        );
    }
}
