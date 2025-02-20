use core::str;
use std::collections::HashMap;

#[cfg(feature = "trace_exec")]
use crate::debug::disassemble_instruction;
use crate::{
    chunk::{Chunk, ChunkWrite, OpCode},
    core::Table,
    object::{Obj, ObjString},
    scanner::{Scanner, Token, TokenType},
    utils::strtod_manual,
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
    pub chunk: Chunk,
    pub strings: Table,
    pub globals: Table,
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
    pub fn new() -> Self {
        Self {
            parser: Parser::default(),
            scanner: Scanner::new_empty(),
            chunk: Chunk::new(),
            strings: Table::init(),
            globals: Table::init(),
        }
    }

    pub fn compile(&mut self, code: &'a str) -> bool {
        self.scanner = Scanner::init_scanner(code.as_bytes());

        loop {
            self.advance();

            while !self.matches(TokenType::Eof) {
                self.declaration();
            }
            // self.expression();
            // self.consume(TokenType::Eof, "Expect end of epxression.".as_bytes());
            self.end_compiler();

            return !self.parser.had_error;
        }
    }

    #[allow(dead_code)]
    fn current_chunk(&self) -> &Chunk {
        &self.chunk
    }

    fn advance(&mut self) {
        self.parser.previous = self.parser.current.clone();

        loop {
            let token = self.scanner.scan_token();
            // println!("{:?}", token);
            self.parser.current = token.clone();
            if self.parser.current.token_type != TokenType::Error {
                break;
            }

            let token_clone = token.clone();
            self.error_at_current(token_clone.start);
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &[u8]) {
        // FIXME: can be removed?
        // if self.parser.current.token_type == TokenType::Eof {
        //     return;
        // }
        if self.parser.current.token_type == token_type {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.parser.current.token_type == token_type
    }

    fn matches(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }
        self.advance();
        return true;
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
    }

    fn binary(&mut self) {
        let operator_type = self.parser.previous.token_type.clone();
        let parse_rule = self.get_rule(operator_type.clone());

        self.parse_precedence(parse_rule.precedence + 1);

        match operator_type {
            TokenType::BangEqual => self.emit_bytes(OpCode::Equal as u8, OpCode::Not as u8),
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal as u8),
            TokenType::Greater => self.emit_byte(OpCode::Greater as u8),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less as u8, OpCode::Not as u8),
            TokenType::Less => self.emit_byte(OpCode::Less as u8),
            TokenType::LessEqual => self.emit_bytes(OpCode::Greater as u8, OpCode::Not as u8),
            TokenType::Plus => self.emit_byte(OpCode::Add as u8),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8),
            _ => return,
        }
    }

    fn literal(&mut self) {
        let operator_type = self.parser.previous.token_type.clone();
        match operator_type {
            TokenType::False => self.emit_byte(OpCode::False as u8),
            TokenType::Nil => self.emit_byte(OpCode::Nil as u8),
            TokenType::True => self.emit_byte(OpCode::True as u8),
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
        let val = strtod_manual(&self.parser.previous.start).unwrap();
        self.emit_constant(Value::Number(val));
    }

    fn string(&mut self) {
        let bytes = &self.parser.previous.start[1..];
        let length = self.parser.previous.length - 2;
        let obj_str = ObjString::new(bytes, length);
        self.strings.set(obj_str.hash, Value::Nil);
        // Strings will have Nil as value, since a string will only be a string. Later on we'll have methods, variables etc
        // that are stored as a string obj for the key and a real Value::{} as the value.

        self.emit_constant(Value::Object(Box::new(Obj::String(obj_str))));
    }

    fn named_variable(&mut self, name: &Token) {
        let arg = self.identifier_constant(name);
        self.emit_bytes(OpCode::GetGlobal as u8, arg);
    }

    fn variable(&mut self) {
        let previous = self.parser.previous.clone();
        self.named_variable(&previous);
    }

    fn unary(&mut self) {
        let operator_type = self.parser.previous.token_type.clone();

        // compile the operand.
        self.parse_precedence(Precedence::Assignment);

        // emit the operator instruction.
        match operator_type {
            TokenType::Bang => self.emit_byte(OpCode::Not as u8),
            TokenType::Minus => self.emit_byte(OpCode::Negate as u8),
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

    fn identifier_constant(&mut self, name: &Token) -> u8 {
        self.make_constant(Value::Object(Box::new(Obj::String(ObjString::new(
            name.start,
            name.length,
        )))))
    }

    fn parse_variable(&mut self, error_message: &[u8]) -> u8 {
        self.consume(TokenType::Identifier, error_message);
        // Cloning here doesn't matter since we just take the tokens bytes and length that we take from the byte array.
        // We do not modify self.parser.previous.
        let previous = self.parser.previous.clone();
        self.identifier_constant(&previous)
    }

    fn define_variable(&mut self, global: u8) {
        self.emit_bytes(OpCode::DefineGlobal as u8, global);
    }

    fn get_rule(&mut self, token_type: TokenType) -> ParseRule<'a> {
        let rules = self.rules();
        *rules.get(&token_type).unwrap()
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn var_declaration(&mut self) {
        let global: u8 = self.parse_variable("Expect variable name.".as_bytes());

        if self.matches(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::Nil as u8);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.".as_bytes(),
        );

        self.define_variable(global);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after expression.".as_bytes(),
        );
        self.emit_byte(OpCode::Pop as u8);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.".as_bytes());
        self.emit_byte(OpCode::Print as u8);
    }

    fn synchronize(&mut self) {
        self.parser.panic_mode = false;

        while self.parser.current.token_type != TokenType::Eof {
            if self.parser.previous.token_type == TokenType::Semicolon {
                return;
            }
            match self.parser.current.token_type {
                TokenType::Class => {}
                TokenType::Fun => {}
                TokenType::Var => {}
                TokenType::For => {}
                TokenType::If => {}
                TokenType::While => {}
                TokenType::Print => {}
                TokenType::Return => {}

                _ => return,
            }

            self.advance();
        }
    }

    fn declaration(&mut self) {
        if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    fn statement(&mut self) {
        if self.matches(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
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
                prefix: Some(Self::unary),
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::BangEqual,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Equality,
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
                infix: Some(Self::binary),
                precedence: Precedence::Equality,
            },
        );
        rules.insert(
            TokenType::Greater,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Comparison,
            },
        );
        rules.insert(
            TokenType::GreaterEqual,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Comparison,
            },
        );
        rules.insert(
            TokenType::Less,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Comparison,
            },
        );
        rules.insert(
            TokenType::LessEqual,
            ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Comparison,
            },
        );
        rules.insert(
            TokenType::Identifier,
            ParseRule {
                prefix: Some(Self::variable),
                infix: None,
                precedence: Precedence::None,
            },
        );
        rules.insert(
            TokenType::String,
            ParseRule {
                prefix: Some(Self::string),
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
                prefix: Some(Self::literal),
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
                prefix: Some(Self::literal),
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
                prefix: Some(Self::literal),
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
