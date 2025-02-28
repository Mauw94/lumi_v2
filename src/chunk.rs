use crate::value::{Value, ValueArray};

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
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    JumpIfFalse,
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
            14 => Some(OpCode::Print),
            15 => Some(OpCode::Pop),
            16 => Some(OpCode::DefineGlobal),
            17 => Some(OpCode::GetGlobal),
            18 => Some(OpCode::SetGlobal),
            19 => Some(OpCode::GetLocal),
            20 => Some(OpCode::SetLocal),
            21 => Some(OpCode::JumpIfFalse),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<i32>,
    pub constants: ValueArray,
}

pub trait ChunkWrite {
    fn new() -> Self;
    fn write_chunk(&mut self, byte: u8, line: i32);
    fn add_constants(&mut self, value: Value, is_final: bool) -> usize;
    fn free(&mut self);
}

impl ChunkWrite for Chunk {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: ValueArray::new(),
        }
    }

    fn write_chunk(&mut self, byte: u8, line: i32) {
        self.code.push(byte);
        self.lines.push(line);
    }

    fn add_constants(&mut self, value: Value, is_final: bool) -> usize {
        self.constants.write_value(value, is_final);
        self.constants.len() - 1
    }

    fn free(&mut self) {
        self.code.clear();
        self.lines.clear();
        self.constants.free();
    }
}
