use crate::{
    chunk::{Chunk, ChunkWrite},
    utils::hash_str,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ObjType {
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    String(ObjString),
    Function(ObjFunction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjString {
    length: usize,
    chars: Vec<u8>,
    pub hash: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjFunction {
    arity: usize,
    chunk: Chunk,
    name: Option<ObjString>,
}

impl ObjString {
    pub fn new(bytes: &[u8], length: usize) -> Self {
        let chars = &bytes[..length];

        Self {
            length,
            chars: chars.to_vec(),
            hash: hash_str(chars, length),
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.chars).expect("Expecting a valid UTF-8 representation.")
    }

    pub fn to_string(&self) -> String {
        std::str::from_utf8(&self.chars)
            .expect("Expecting a valid UTF-8 representation.")
            .to_string()
    }
}

impl ObjFunction {
    pub fn new() -> Self {
        Self {
            arity: 0,
            chunk: Chunk::new(),
            name: None,
        }
    }
}
