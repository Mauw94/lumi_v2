use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct Scanner<'a> {
    pub start: &'a [u8],
    pub current: &'a [u8],
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    pub token_type: TokenType,
    pub start: &'a [u8],
    pub length: usize,
    pub line: usize,
}

impl<'a> Token<'a> {
    pub fn default() -> Self {
        Self {
            token_type: TokenType::Nil,
            start: &[],
            length: 0,
            line: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier,
    String,
    Number,
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Error,
    Eof,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::LeftParen => write!(f, "LeftParen"),
            TokenType::RightParen => write!(f, "RightParen"),
            TokenType::LeftBrace => write!(f, "LeftBrace"),
            TokenType::RightBrace => write!(f, "RightBrace"),
            TokenType::LeftBracket => write!(f, "LeftBracket"),
            TokenType::RightBracket => write!(f, "RightBracket"),
            TokenType::Comma => write!(f, "Comma"),
            TokenType::Dot => write!(f, "Dot"),
            TokenType::Minus => write!(f, "Minus"),
            TokenType::Plus => write!(f, "Plus"),
            TokenType::Semicolon => write!(f, "Semicolon"),
            TokenType::Slash => write!(f, "Slash"),
            TokenType::Star => write!(f, "Star"),
            TokenType::Bang => write!(f, "Bang"),
            TokenType::BangEqual => write!(f, "BangEqual"),
            TokenType::Equal => write!(f, "Equal"),
            TokenType::EqualEqual => write!(f, "EqualEqual"),
            TokenType::Greater => write!(f, "Greater"),
            TokenType::GreaterEqual => write!(f, "GreaterEqual"),
            TokenType::Less => write!(f, "Less"),
            TokenType::LessEqual => write!(f, "LessEqual"),
            TokenType::Identifier => write!(f, "Identifier"),
            TokenType::String => write!(f, "String"),
            TokenType::Number => write!(f, "Number"),
            TokenType::And => write!(f, "And"),
            TokenType::Class => write!(f, "Class"),
            TokenType::Else => write!(f, "Else"),
            TokenType::False => write!(f, "False"),
            TokenType::For => write!(f, "For"),
            TokenType::Fun => write!(f, "Fun"),
            TokenType::If => write!(f, "If"),
            TokenType::Nil => write!(f, "Nil"),
            TokenType::Or => write!(f, "Or"),
            TokenType::Print => write!(f, "Print"),
            TokenType::Return => write!(f, "Return"),
            TokenType::Super => write!(f, "Super"),
            TokenType::This => write!(f, "This"),
            TokenType::True => write!(f, "True"),
            TokenType::Var => write!(f, "Var"),
            TokenType::While => write!(f, "While"),
            TokenType::Error => write!(f, "Error()"),
            TokenType::Eof => write!(f, "EOF"),
        }
    }
}

impl<'a> Scanner<'a> {
    pub fn new_empty() -> Self {
        Self {
            start: &[],
            current: &[],
            line: 0,
        }
    }

    pub fn init_scanner(source: &'a [u8]) -> Self {
        Self {
            start: source,
            current: source,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let c = self.advance();

        if c.is_alphabetic() {
            return self.identifier(c);
        }
        if c.is_digit(10) {
            return self.number();
        }
        return match c {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            '[' => self.make_token(TokenType::LeftBracket),
            ']' => self.make_token(TokenType::RightBracket),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            ';' => self.make_token(TokenType::Semicolon),
            '+' => self.make_token(TokenType::Plus),
            '*' => self.make_token(TokenType::Star),
            '/' => self.make_token(TokenType::Slash),
            '-' => self.make_token(TokenType::Minus),
            '!' => {
                if self.match_next('=') {
                    return self.make_token(TokenType::BangEqual);
                } else {
                    return self.make_token(TokenType::Bang);
                }
            }
            '=' => {
                if self.match_next('=') {
                    return self.make_token(TokenType::EqualEqual);
                } else {
                    return self.make_token(TokenType::Equal);
                }
            }
            '<' => {
                if self.match_next('=') {
                    return self.make_token(TokenType::LessEqual);
                } else {
                    return self.make_token(TokenType::Less);
                }
            }
            '>' => {
                if self.match_next('=') {
                    return self.make_token(TokenType::GreaterEqual);
                } else {
                    return self.make_token(TokenType::Greater);
                }
            }
            '"' => return self.string(),
            _ => self.error_token("Unexpected character.".to_string()),
        };
    }

    fn is_at_end(&mut self) -> bool {
        self.current.len() == 0 || self.current[0] == b'\0'
    }

    fn make_token(&self, token_type: TokenType) -> Token<'a> {
        Token {
            token_type,
            start: self.start,
            length: self.start.len() - self.current.len(),
            line: self.line,
        }
    }

    fn error_token(&self, message: String) -> Token<'a> {
        Token {
            token_type: TokenType::Error,
            start: self.current,
            length: message.len(),
            line: self.line,
        }
    }

    fn advance(&mut self) -> char {
        let c = self.current[0] as char;
        self.current = &self.current[1..];
        return c;
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.current[0] as char != expected {
            return false;
        }
        self.current = &self.current[1..];
        true
    }

    fn peek(&mut self) -> char {
        if self.current.len() > 0 {
            self.current[0] as char
        } else {
            return '\0';
        }
    }

    fn peek_next(&mut self) -> char {
        if self.is_at_end() {
            return '\0';
        }

        self.current[1] as char
    }

    fn skip_whitespace(&mut self) {
        loop {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                    // break; If we break we skip over Windows-style newlines (\r\n).
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                    break;
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                    break;
                }
                _ => break,
            }
        }
    }

    fn string(&mut self) -> Token<'a> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.".to_string());
        }

        self.advance();
        return self.make_token(TokenType::String);
    }

    fn number(&mut self) -> Token<'a> {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    fn identifier(&mut self, first: char) -> Token<'a> {
        let mut keyword: String = String::new();
        keyword.push(first);

        while self.peek().is_alphabetic() || self.peek().is_digit(10) {
            keyword.push(self.current[0] as char);
            self.advance();
        }
        self.make_token(self.identifier_type(keyword.as_str()))
    }

    fn identifier_type(&self, keyword: &str) -> TokenType {
        let mut keywords = HashMap::new();
        keywords.insert("and", TokenType::And);
        keywords.insert("or", TokenType::Or);
        keywords.insert("if", TokenType::If);
        keywords.insert("else", TokenType::Else);
        keywords.insert("false", TokenType::False);
        keywords.insert("true", TokenType::True);
        keywords.insert("fun", TokenType::Fun);
        keywords.insert("nil", TokenType::Nil);
        keywords.insert("print", TokenType::Print);
        keywords.insert("return", TokenType::Return);
        keywords.insert("while", TokenType::While);
        keywords.insert("var", TokenType::Var);
        keywords.insert("class", TokenType::Class);
        keywords.insert("super", TokenType::Super);
        keywords.insert("for", TokenType::For);
        keywords.insert("this", TokenType::This);

        if keywords.contains_key(keyword) {
            keywords.get(keyword).unwrap().clone()
        } else {
            return TokenType::Identifier;
        }
    }
}
