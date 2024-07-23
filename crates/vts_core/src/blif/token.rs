use crate::bytescanner::Scanner;

use super::buffer::{BlifBuffer, BlifLines, BytePos, Span};
use super::error::Result;

pub(super) trait BlifChar {
    fn is_line_whitespace(&self) -> bool;

    fn is_token_terminator(&self) -> bool;

    fn is_cube_input(&self) -> bool;

    fn is_cube_output(&self) -> bool;
}

impl BlifChar for u8 {
    /// Returns `true` if the byte is whitespace (excluding newlines),
    /// else `false`.
    #[inline]
    fn is_line_whitespace(&self) -> bool {
        // TODO: Confirm this with other tools.
        matches!(*self, b'\t' | b'\x0C' | b'\r' | b' ')
    }

    /// Returns `true` if the byte would end a single token, else `false`.
    #[inline]
    fn is_token_terminator(&self) -> bool {
        // TODO: Confirm this with other tools.
        matches!(*self, b'\t' | b'\x0C' | b'\r' | b' ' | b'\n' | b'#')
    }

    /// Returns `true` if the byte is a valid cube input value, else `false`.
    #[inline]
    fn is_cube_input(&self) -> bool {
        matches!(*self, b'0' | b'1' | b'-')
    }

    /// Returns `true` if the byte is a valid cube output value, else `false`.
    #[inline]
    fn is_cube_output(&self) -> bool {
        matches!(*self, b'0' | b'1')
    }
}

pub(super) trait BlifScanner<'a> {
    fn eat_line_whitespace(&mut self) -> &'a [u8];

    fn eat_non_whitespace(&mut self) -> &'a [u8];

    fn eat_token(&mut self) -> &'a [u8];

    fn at_token_terminator(&self) -> bool;
}

impl<'a> BlifScanner<'a> for Scanner<'a> {
    /// Consume whitespace, excluding newlines.
    #[inline]
    fn eat_line_whitespace(&mut self) -> &'a [u8] {
        self.eat_while(BlifChar::is_line_whitespace)
    }

    /// Consume non-whitespace.
    #[inline]
    fn eat_non_whitespace(&mut self) -> &'a [u8] {
        self.eat_until(u8::is_ascii_whitespace)
    }

    /// Consume a single token.
    #[inline]
    fn eat_token(&mut self) -> &'a [u8] {
        self.eat_until(BlifChar::is_token_terminator)
    }

    /// Returns `true` if the scanner is currently at a token terminator, else
    /// `false`.
    fn at_token_terminator(&self) -> bool {
        self.at(BlifChar::is_token_terminator)
    }
}

/// The scanned token kind.
#[derive(Debug, PartialEq)]
pub(super) enum TokenKind {
    /// A command.
    Command,
    /// A cube.
    Cube,
}

// A scanned token.
#[derive(Debug, PartialEq)]
pub(super) struct Token {
    pub(super) kind: TokenKind,
    pub(super) trivia: Vec<Span>,
    pub(super) extent: Span,
}

/// An iterator over scanned tokens.
pub(super) struct Tokenizer<'a> {
    lines: BlifLines<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Start a new tokenizer over `lines`.
    fn new(lines: BlifLines<'a>) -> Self {
        Self { lines }
    }

    /// Pre-process and start a new tokenizer.
    pub fn new_preprocess(buffer: &'a BlifBuffer) -> Self {
        Self::new(buffer.preprocess())
    }
}

/// `Command ::= "." cmd-name (S+ arg0 (S+ argn)*)?`
fn tokenize_command_line(line: &[u8], start_pos: BytePos) -> Result<Token> {
    let mut scanner = Scanner::new(line);
    scanner.expect(b'.');
    scanner.eat_token();
    let mut token_end = scanner.cursor();
    if token_end < 2 {
        // TODO(rikus): Report empty command name.
        panic!("empty command name");
    }
    scanner.eat_whitespace();
    // Start `trivia` with a span of the command name.
    let mut trivia = vec![Span::new(start_pos, token_end)];
    let mut trivia_start = scanner.cursor();
    // NOTE: Trivia beyond the command name is assumed to be optional.
    while !scanner.done() && !scanner.at_token_terminator() {
        scanner.eat_token();
        token_end = scanner.cursor();
        trivia.push(Span::new_rebased_range(start_pos, trivia_start, token_end));
        scanner.eat_whitespace();
        trivia_start = scanner.cursor();
    }
    Ok(Token {
        kind: TokenKind::Command,
        trivia,
        extent: Span::new(start_pos, token_end),
    })
}

/// `Cube ::= ("0" | "1" | "-")+ S+ ("0" | "1")`
fn tokenize_cube_line(line: &[u8], start_pos: BytePos) -> Result<Token> {
    let mut scanner = Scanner::new(line);
    scanner.expect(BlifChar::is_cube_input);
    scanner.eat_while(BlifChar::is_cube_input);
    let input_end = scanner.cursor();
    if scanner.eat_whitespace().is_empty() {
        // TODO(rikus): Report expected whitespace.
        panic!("expected whitespace");
    }
    let output_start = scanner.cursor();
    // NOTE: Report multi-bit outputs as errors.
    scanner.eat_while(BlifChar::is_cube_output);
    let token_end = scanner.cursor();
    if output_start == token_end {
        // TODO(rikus): Report empty output.
        panic!("expected '0' or '1'");
    }
    scanner.eat_whitespace();
    if scanner.at(b'#') {
        scanner.jump_end();
    }
    if !scanner.done() {
        // TODO(rikus): Report unexpected bytes.
        panic!("unexpected {:?}", &line[scanner.cursor()..]);
    }
    Ok(Token {
        kind: TokenKind::Cube,
        trivia: Vec::from_iter([
            Span::new(start_pos, input_end),
            Span::new_rebased_range(start_pos, output_start, token_end),
        ]),
        extent: Span::new(start_pos, token_end),
    })
}

impl Iterator for Tokenizer<'_> {
    type Item = Result<Token>;

    /// Get the next line's tokens from the tokenizer.
    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            let line_bytes = line.get_bytes();
            let mut scanner = Scanner::new(line_bytes);
            match scanner.peek() {
                Some(b'.') => {
                    return Some(tokenize_command_line(line_bytes, line.start_pos()));
                }
                Some(logic) if logic.is_cube_input() => {
                    return Some(tokenize_cube_line(line_bytes, line.start_pos()));
                }
                Some(b'#') => {
                    // Comment lines are ignored.
                    continue;
                }
                Some(unexpected) => {
                    // TODO(rikus): Report unexpected byte.
                    panic!("unexpected {:?}", unexpected);
                }
                None => {
                    // TODO(rikus): Should this be `unreachable!`?
                    panic!("unexpected empty line");
                }
            }
        }
        None
    }
}
