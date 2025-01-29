use core::str;
use std::collections::HashMap;

use crate::{
    chunk::{Chunk, OpCode, Write},
    debug::disassemble_chunk,
    scanner::{Scanner, Token, TokenType},
    value::Value,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

type ParseFn<'a> = Option<fn(&mut Compiler<'a>)>;

#[derive(Debug, Clone, Copy)]
struct ParseRule<'a> {
    prefix: ParseFn<'a>,
    infix: ParseFn<'a>,
    precedence: Precedence,
}

#[derive(Debug, Clone)]
struct Parser<'a> {
    current: Token<'a>,
    previous: Token<'a>,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    fn default() -> Self {
        Self {
            current: Token::default(),
            previous: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }
}

#[derive(Debug)]
pub struct Compiler<'a> {
    parser: Parser<'a>,
    scanner: Scanner<'a>,
    chunk: &'a mut Chunk,
}

use std::ops::Add;

impl Add<u8> for Precedence {
    type Output = Precedence;

    fn add(self, other: u8) -> Precedence {
        match self as u8 + other {
            0 => Precedence::None,
            1 => Precedence::Assignment,
            2 => Precedence::Or,
            3 => Precedence::And,
            4 => Precedence::Equality,
            5 => Precedence::Comparison,
            6 => Precedence::Term,
            7 => Precedence::Factor,
            8 => Precedence::Unary,
            9 => Precedence::Call,
            10 => Precedence::Primary,
            _ => Precedence::None, // Default case
        }
    }
}

impl<'a> Compiler<'a> {
    pub fn new(code: &'a str, chunk: &'a mut Chunk) -> Self {
        Self {
            parser: Parser::default(),
            scanner: Scanner::init_scanner(code.as_bytes()),
            chunk,
        }
    }

    pub fn compile(&mut self) -> bool {
        loop {
            self.advance();
            self.expression();
            println!("CHUNK constants: {:?}", self.chunk.constants);
            println!("CHUNK CODE: {:?}", self.chunk.code);
            self.consume(TokenType::Eof, "Expect end of epxression.".as_bytes());
            self.end_compiler();

            return !self.parser.had_error;
        }
    }

    fn current_chunk(&self) -> &Chunk {
        &self.chunk
    }

    fn advance(&mut self) {
        self.parser.previous = self.parser.current.clone();

        loop {
            let token = self.scanner.scan_token();
            self.parser.current = token.clone();
            if self.parser.current.token_type != TokenType::Error {
                break;
            }

            let token_clone = token.clone();
            self.error_at_current(token_clone.start);
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &[u8]) {
        if self.parser.current.token_type == TokenType::Eof {
            return;
        }
        if self.parser.current.token_type == token_type {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn emit_byte(&mut self, byte: u8) {
        self.chunk
            .write_chunk(byte, self.parser.previous.line as i32);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return as u8);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.chunk.add_constants(value);
        if constant as u8 > u8::MAX {
            self.error("Too many constants in one chunk.".as_bytes());
            return 0;
        }

        constant as u8
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(OpCode::Constant as u8, constant);
    }

    fn end_compiler(&mut self) {
        self.emit_return();

        #[cfg(feature = "trace_exec")]
        if !self.parser.had_error {
            disassemble_chunk(self.current_chunk().clone(), "code");
        }
    }

    fn binary(&mut self) {
        let operator_type = self.parser.previous.token_type.clone();
        let parse_rule = self.get_rule(operator_type.clone());

        self.parse_precedence(parse_rule.precedence + 1);

        match operator_type {
            TokenType::Plus => self.emit_byte(OpCode::Add as u8),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8),
            _ => return,
        }
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(
            TokenType::RightParen,
            "Expect ')' after expression.".as_bytes(),
        );
    }

    fn number(&mut self) {
        // println!("{:?}", token.start);
        let val = Self::strtod_manual(&self.parser.previous.start).unwrap();
        self.emit_constant(Value::Number(val));
    }

    // TODO: move to some utils
    fn strtod_manual(input: &[u8]) -> Option<f64> {
        let input_str = str::from_utf8(input).ok()?;

        // Extract the numeric prefix
        let numeric_part: String = input_str.chars().take_while(|c| c.is_digit(10)).collect();

        if numeric_part.is_empty() {
            None
        } else {
            let parsed = numeric_part.parse::<f64>().ok()?;
            Some(parsed)
        }
    }

    fn unary(&mut self) {
        let operator_type = self.parser.previous.token_type.clone();

        // compile the operand.
        self.parse_precedence(Precedence::Assignment);

        // emit the operator instruction.
        match operator_type {
            TokenType::Minus => {
                self.emit_byte(OpCode::Negate as u8);
                return;
            }
            _ => return,
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        if let Some(prefix) = self
            .get_rule(self.parser.previous.token_type.clone())
            .prefix
        {
            prefix(self);

            while precedence
                <= self
                    .get_rule(self.parser.current.token_type.clone())
                    .precedence
            {
                self.advance();
                if let Some(infix) = self.get_rule(self.parser.previous.token_type.clone()).infix {
                    infix(self);
                }
            }
        } else {
            self.error("Expect expression.".as_bytes());
        }
    }

    fn get_rule(&mut self, token_type: TokenType) -> ParseRule<'a> {
        let rules = self.rules();
        *rules.get(&token_type).unwrap()
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn error_at_current(&mut self, message: &[u8]) {
        self.error_at(&self.parser.current.clone(), message);
    }

    fn error(&mut self, message: &[u8]) {
        self.error_at(&self.parser.previous.clone(), message);
    }

    fn error_at(&mut self, token: &Token, message: &[u8]) {
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;
        eprint!("[line {}] Error", token.line);

        match token.token_type {
            TokenType::Eof => {
                eprint!(" at end");
            }
            TokenType::Error => {
                // Do nothing
            }
            _ => {
                eprint!(
                    " at '{:?}'",
                    std::str::from_utf8(&token.start[0..token.length]).expect("Invalid UTF-8.")
                );
            }
        }

        eprintln!(
            ": {}",
            std::str::from_utf8(message).expect("Invalid UTF-8.")
        );
        self.parser.had_error = true;
    }

    fn rules(&self) -> HashMap<TokenType, ParseRule<'a>> {
        let mut rules = HashMap::new();

        rules.insert(
            TokenType::LeftParen,
            ParseRule {
                prefix: Some(Self::grouping),
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::RightParen,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::LeftBrace,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::RightBrace,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Comma,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Dot,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Minus,
            ParseRule {
                prefix: Some(Self::unary),
                infix: Some(Self::binary),
                precedence: Precedence::Term,
            },
        );
        rules.insert(
            TokenType::Plus,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Term,
            },
        );
        rules.insert(
            TokenType::Semicolon,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Slash,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Factor,
            },
        );
        rules.insert(
            TokenType::Star,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Factor,
            },
        );
        rules.insert(
            TokenType::Bang,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::BangEqual,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Equal,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::EqualEqual,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Greater,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::GreaterEqual,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::LeftBrace,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::LessEqual,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Identifier,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::String,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Number,
            ParseRule {
                prefix: Some(Self::number),
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::And,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Class,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Else,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::False,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::For,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Fun,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::If,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Nil,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Or,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Print,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Return,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Super,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::This,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::True,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Var,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::While,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::Error,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );

        rules.insert(
            TokenType::Eof,
            ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        );

        rules
    }
}
