//! The "AST" for a BLIF file.

use super::buffer::{BlifBuffer, Span};
use super::error::{Error, Result};

/// A BLIF "cube".
pub struct Cube {
    input: Vec<Span>,
    output: Span,
}

/// An assignment, `lvalue=rvalue`.
pub struct Assignment {
    /// The LHS of the '='.
    pub lvalue: Span,
    /// The RHS of the '='.
    pub rvalue: Span,
}

/// A BLIF command.
pub enum Command {
    /// `Model ::= ".model" (S+ model-name)?`
    Model { name: Option<Span>, span: Span },
    /// `Inputs ::= ".inputs" S+ input0 (S+ inputN)*`
    Inputs { inputs: Vec<Span>, span: Span },
    /// `Outputs ::= ".outputs" S+ output0 (S+ outputN)*`
    Outputs { outputs: Vec<Span>, span: Span },
    /// `Names ::= ".names" S+ input0 (S+ inputN)* S+ output`
    Names {
        inputs: Vec<Span>,
        output: Span,
        span: Span,
    },
    /// `Latch ::= ".latch" S+ input S+ output (S+ type S+ ctrl)? (S+ init)?`
    Latch {
        input: Span,
        output: Span,
        ty: Option<Span>,
        ctrl: Option<Span>,
        init: Option<Span>,
        span: Span,
    },
    /// `Subckt ::= ".subckt" S+ model-name S+ formal-actual-list`
    ///
    /// `formal-actual-list ::= formal-actual0 (S+ formal-actualN)*`
    ///
    /// `formal-actual ::= lvalue "=" rvalue`
    Subckt {
        name: Span,
        formal_actual: Vec<Assignment>,
        span: Span,
    },
    /// `End ::= ".end"`
    End { span: Span },
}

impl Command {
    /// Try to parse a `.model` line.
    #[inline]
    fn parse_model<I>(cmd_name: Span, mut trivia: I, buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let model_name = trivia.next();
        if let Some(unexpected) = trivia.next() {
            // TODO(rikus): Report unexpected trailing trivia.
            panic!("unexpected {:?}", buffer.view(unexpected));
        }
        let last_token = model_name.unwrap_or(cmd_name);
        Ok(Self::Model {
            span: Span::new_token_range(&cmd_name, &last_token),
            name: model_name,
        })
    }

    /// Try to parse a `.inputs` line.
    #[inline]
    fn parse_inputs<I>(cmd_name: Span, trivia: I, _buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let inputs = trivia.collect::<Vec<_>>();
        // TODO(rikus): Are empty `.inputs` an error?
        let last_token = inputs.iter().last().copied().unwrap_or(cmd_name);
        Ok(Self::Inputs {
            span: Span::new_token_range(&cmd_name, &last_token),
            inputs,
        })
    }

    /// Try to parse a `.outputs` line.
    #[inline]
    fn parse_outputs<I>(cmd_name: Span, trivia: I, _buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let outputs = trivia.collect::<Vec<_>>();
        // TODO(rikus): Are empty `.outputs` an error?
        let last_token = outputs.iter().last().copied().unwrap_or(cmd_name);
        Ok(Self::Outputs {
            span: Span::new_token_range(&cmd_name, &last_token),
            outputs,
        })
    }

    /// Try to parse a `.names` line.
    #[inline]
    fn parse_names<I>(cmd_name: Span, trivia: I, _buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let mut inputs = trivia.collect::<Vec<_>>();
        let output = inputs.pop().expect("expected `.names` output");
        // TODO(rikus): Report missing input.
        if inputs.is_empty() {
            panic!("expected `.names` input");
        }
        Ok(Self::Names {
            span: Span::new_token_range(&cmd_name, &output),
            inputs,
            output,
        })
    }

    /// Try to parse a `.latch` line.
    #[inline]
    fn parse_latch<I>(cmd_name: Span, mut trivia: I, buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let input = trivia.next().expect("expected latch input");
        let output = trivia.next().expect("expected latch output");
        let (ty, ctrl, init) = match (trivia.next(), trivia.next(), trivia.next()) {
            // <ty> and <ctrl> is given, with optional <init>.
            (Some(ty), Some(ctrl), init) => (Some(ty), Some(ctrl), init),
            // <ty> and <ctrl> is not given, but <init> maybe is.
            (init, None, None) => (None, None, init),
            (None, Some(_), _) | (_, None, Some(_)) => {
                // `Some` can never come after `None`.
                unreachable!();
            }
        };
        if let Some(unexpected) = trivia.next() {
            // TODO(rikus): Report unexpected trivia.
            panic!("unexpected {:?}", buffer.view(unexpected));
        }
        let last_token = init.unwrap_or_else(|| ctrl.unwrap_or_else(|| ty.unwrap_or(output)));
        Ok(Self::Latch {
            span: Span::new_token_range(&cmd_name, &last_token),
            input,
            output,
            ty,
            ctrl,
            init,
        })
    }

    /// Try to parse a `.subckt` line.
    fn parse_subckt<I>(cmd_name: Span, mut trivia: I, buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let model_name = trivia.next().expect("expected `.subckt` model name");
        let formal_actual = trivia.try_fold(Vec::new(), |mut list, assignment| {
            // TODO(rikus): Report expected assignment.
            let needle_pos = buffer
                .view(assignment)
                .iter()
                .position(|&b| b == b'=')
                .expect("expected `formal=actual` pair");
            let rvalue_len = assignment.end_pos().0 - needle_pos;
            list.push(Assignment {
                lvalue: Span::new(assignment.start_pos(), needle_pos),
                rvalue: Span::new_rebased_range(assignment.start_pos(), needle_pos, rvalue_len),
            });
            Ok::<_, Error>(list)
        })?;
        // TODO(rikus): Is empty `formal=actual` an error?
        let last_token = formal_actual
            .iter()
            .last()
            .map(|assignment| assignment.rvalue)
            .unwrap_or(model_name);
        Ok(Self::Subckt {
            span: Span::new_token_range(&cmd_name, &last_token),
            name: model_name,
            formal_actual,
        })
    }

    /// Try to parse trivia as a command.
    ///
    /// Returns `Ok(Command)` on success or `Err(Error)` on failure.
    /// Panics if:
    /// - `trivia` does not yield at least a single token
    /// - the first token does not start with a `.`
    pub fn parse_trivia<I>(mut trivia: I, buffer: &BlifBuffer) -> Result<Self>
    where
        I: Iterator<Item = Span>,
    {
        let name_extent = trivia
            .next()
            .expect("trivia should yield at least a single token");
        let cmd_name = buffer.view(name_extent);
        assert!(cmd_name.starts_with(b"."));
        match &cmd_name[1..] {
            b"model" => Self::parse_model(name_extent, trivia, buffer),
            b"inputs" => Self::parse_inputs(name_extent, trivia, buffer),
            b"outputs" => Self::parse_outputs(name_extent, trivia, buffer),
            b"names" => Self::parse_names(name_extent, trivia, buffer),
            b"latch" => Self::parse_latch(name_extent, trivia, buffer),
            b"subckt" => Self::parse_subckt(name_extent, trivia, buffer),
            b"end" => {
                if let Some(unexpected) = trivia.next() {
                    // TODO(rikus): Report unexpected trivia.
                    panic!("unexpected {:?}", buffer.view(unexpected));
                }
                Ok(Self::End { span: name_extent })
            }
            [] => {
                // TODO(rikus): Report empty command.
                panic!("empty command");
            }
            unknown => {
                // TODO(rikus): Report unknown commands and potentially known
                // but unsupported commands.
                panic!("unknown command {:?}", unknown);
            }
        }
    }

    /// The full extent of the command.
    pub fn span(&self) -> &Span {
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
