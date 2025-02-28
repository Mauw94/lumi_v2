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

const MAX_LOCALS: u8 = u8::MAX;

#[derive(Debug)]
struct TinyCompiler<'a> {
    // locals: [Local<'a>; MAX_LOCALS as usize],
    locals: Vec<Local<'a>>,
    local_count: usize,
    scope_depth: usize,
    is_final: bool,
}

impl<'a> TinyCompiler<'a> {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            local_count: 0,
            scope_depth: 0,
            is_final: false,
        }
    }
}

#[derive(Debug)]
struct Local<'a> {
    name: Token<'a>,
    depth: i8,
}

impl<'a> Local<'a> {
    pub fn new(name: Token<'a>, depth: i8) -> Self {
        Self { name, depth }
    }
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
    current: TinyCompiler<'a>,
    pub chunk: Chunk,
    pub strings: Table,
    pub globals: Table,
    can_assign: bool,
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
            current: TinyCompiler::new(),
            chunk: Chunk::new(),
            strings: Table::init(),
            globals: Table::init(),
            can_assign: false,
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

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
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

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::Loop as u8);

        let offset = self.current_chunk().code.len() - loop_start + 2;
        if offset as u16 > u16::MAX {
            self.error("Loop body too large.".as_bytes());
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        return self.current_chunk().code.len() - 2;
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return as u8);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.chunk.add_constants(value, self.current.is_final);
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

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;
        if jump as u16 > u16::MAX {
            self.error("Too much code to jump over.".as_bytes());
        }

        self.current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk().code[offset + 1] = (jump & 0xff) as u8;
    }

    fn end_compiler(&mut self) {
        self.emit_return();
    }

    fn begin_scope(&mut self) {
        self.current.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.current.scope_depth -= 1;

        while self.current.local_count > 0
            && self.current.locals[self.current.local_count - 1].depth
                > self.current.scope_depth as i8
        {
            self.emit_byte(OpCode::Pop as u8);
            self.current.locals.remove(self.current.local_count - 1);
            self.current.local_count -= 1;
        }
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

    fn or(&mut self) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        let end_jump = self.emit_jump(OpCode::Jump as u8);

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::Pop as u8);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
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
        let mut arg = self.resolve_local(name);
        let get_op: u8;
        let set_op: u8;

        if arg != -1 {
            get_op = OpCode::GetLocal as u8;
            set_op = OpCode::SetLocal as u8;
        } else {
            arg = self.identifier_constant(name) as i8;
            get_op = OpCode::GetGlobal as u8;
            set_op = OpCode::SetGlobal as u8;
        }

        if self.can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op, arg as u8);
        } else {
            self.emit_bytes(get_op, arg as u8);
        }
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
            self.can_assign = precedence <= Precedence::Assignment;
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

                if self.can_assign && self.matches(TokenType::Equal) {
                    self.error("Invalid assignment target.".as_bytes());
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

    fn identifiers_equal(&self, a: &Token, b: &Token) -> bool {
        if a.length != b.length {
            return false;
        }

        if &a.start[..a.length] != &b.start[..b.length] {
            return false;
        }

        true
    }

    fn resolve_local(&mut self, previous: &Token) -> i8 {
        for i in 0..self.current.local_count {
            let local = self.current.locals.get(i).unwrap();

            if self.identifiers_equal(&previous, &local.name) {
                if local.depth == -1 {
                    self.error("Can't reqad local variable in its own initializer.".as_bytes());
                }
                return i as i8;
            }
        }

        return -1;
    }

    fn add_local(&mut self, name: Token<'a>) {
        if self.current.local_count == MAX_LOCALS as usize {
            self.error("Too many local variables in function.".as_bytes());
        }

        let local = Local::new(name, -1);
        self.current.local_count += 1;
        self.current.locals.push(local);
    }

    fn declare_variable(&mut self) {
        if self.current.scope_depth == 0 {
            return;
        }

        let previous = self.parser.previous.clone();

        for i in 0..self.current.local_count {
            let local = self.current.locals.get(i).unwrap();
            if local.depth != -1 && local.depth < self.current.scope_depth as i8 {
                break;
            }

            if self.identifiers_equal(&previous, &local.name) {
                self.error("Already a variable with this name in this scope.".as_bytes());
            }
        }

        self.add_local(previous);
    }

    fn parse_variable(&mut self, error_message: &[u8]) -> u8 {
        self.consume(TokenType::Identifier, error_message);
        self.declare_variable();

        if self.current.scope_depth > 0 {
            return 0;
        }

        // Cloning here doesn't matter since we just take the tokens bytes and length that we took from the byte array.
        // We do not modify self.parser.previous.
        let previous = self.parser.previous.clone();
        self.identifier_constant(&previous)
    }

    fn mark_initialized(&mut self) {
        self.current.locals[self.current.local_count - 1].depth = self.current.scope_depth as i8;
    }

    fn define_variable(&mut self, global: u8) {
        if self.current.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(OpCode::DefineGlobal as u8, global);
    }

    fn and(&mut self) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse as u8);

        self.emit_byte(OpCode::Pop as u8);
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump);
    }

    fn get_rule(&mut self, token_type: TokenType) -> ParseRule<'a> {
        let rules = self.rules();
        *rules.get(&token_type).unwrap()
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.".as_bytes());
    }

    fn var_declaration(&mut self) {
        self.current.is_final = self.matches(TokenType::Final);
        // FIXME: emit final opcode here
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

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.".as_bytes());
        if self.matches(TokenType::Semicolon) {
            // no initializer.
        } else if self.matches(TokenType::Let) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().code.len();
        let mut exit_jump = 0;
        if !self.matches(TokenType::Semicolon) {
            self.expression();
            self.consume(
                TokenType::Semicolon,
                "Expect ';' after loop condition.".as_bytes(),
            );

            exit_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
            self.emit_byte(OpCode::Pop as u8);
        }

        if !self.matches(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump as u8);
            let increment_start = self.current_chunk().code.len();
            self.expression();
            self.emit_byte(OpCode::Pop as u8);
            self.consume(
                TokenType::RightParen,
                "Expect ')' after for clauses.".as_bytes(),
            );

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if exit_jump != 0 {
            self.patch_jump(exit_jump);
            self.emit_byte(OpCode::Pop as u8);
        }

        self.end_scope();
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.".as_bytes());
        self.expression();
        self.consume(
            TokenType::RightParen,
            "Expect ')' after condition.".as_bytes(),
        );

        let then_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        self.emit_byte(OpCode::Pop as u8);
        self.statement();

        let else_jump = self.emit_jump(OpCode::Jump as u8);

        self.patch_jump(then_jump);
        self.emit_byte(OpCode::Pop as u8);

        if self.matches(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.".as_bytes());
        self.emit_byte(OpCode::Print as u8);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.".as_bytes());
        self.expression();
        self.consume(
            TokenType::RightParen,
            "Expect ')' after condition.".as_bytes(),
        );

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        self.emit_byte(OpCode::Pop as u8);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(OpCode::Pop as u8);
    }

    // FIXME: doesn't seem to sync up properly.
    fn synchronize(&mut self) {
        self.parser.panic_mode = false;

        while self.parser.current.token_type != TokenType::Eof {
            if self.parser.previous.token_type == TokenType::Semicolon {
                return;
            }
            match self.parser.current.token_type {
                TokenType::Class => {}
                TokenType::Fun => {}
                TokenType::Let => {}
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
        if self.matches(TokenType::Let) {
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
        } else if self.matches(TokenType::For) {
            self.for_statement();
        } else if self.matches(TokenType::If) {
            self.if_statement();
        } else if self.matches(TokenType::While) {
            self.while_statement();
        } else if self.matches(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
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
                infix: Some(Self::and),
                precedence: Precedence::And,
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
                infix: Some(Self::or),
                precedence: Precedence::Or,
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
            TokenType::Let,
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
