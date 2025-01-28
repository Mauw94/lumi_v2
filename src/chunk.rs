use crate::{
    memory::grow_capacity,
    value::{Value, ValueArray},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Constant,
    Nil,
    True,
    False,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Return,
}

impl OpCode {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(OpCode::Constant),
            1 => Some(OpCode::Nil),
            2 => Some(OpCode::True),
            3 => Some(OpCode::False),
            4 => Some(OpCode::Equal),
            5 => Some(OpCode::Greater),
            6 => Some(OpCode::Less),
            7 => Some(OpCode::Add),
            8 => Some(OpCode::Subtract),
            9 => Some(OpCode::Multiply),
            10 => Some(OpCode::Divide),
            11 => Some(OpCode::Not),
            12 => Some(OpCode::Negate),
            13 => Some(OpCode::Return),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub count: usize,
    pub capacity: usize,
    pub code: Vec<u8>,
    pub lines: Vec<i32>,
    pub constants: ValueArray,
}

pub trait Write {
    fn new() -> Self;
    fn write_chunk(&mut self, byte: u8, line: i32);
    fn add_constants(&mut self, value: Value) -> usize;
    fn free(&mut self);
}

impl Write for Chunk {
    fn new() -> Self {
        Self {
            count: 0,
            capacity: 0,
            code: Vec::new(),
            lines: Vec::new(),
            constants: ValueArray::new(),
        }
    }

    fn write_chunk(&mut self, byte: u8, line: i32) {
        if self.capacity <= self.count {
            self.capacity = grow_capacity(self.capacity);
            self.code.reserve(self.capacity - self.code.len());
            self.lines.reserve(self.capacity - self.lines.len());
        }

        self.code.push(byte);
        self.lines.push(line);
        self.count += 1;
    }

    fn add_constants(&mut self, value: Value) -> usize {
        self.constants.write_value(value);
        self.constants.len() - 1
    }

    fn free(&mut self) {
        self.code.clear();
        self.lines.clear();
        self.constants.free();
    }
}
