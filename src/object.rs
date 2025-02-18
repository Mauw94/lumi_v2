use std::rc::Rc;

use crate::utils::hash_str;

#[derive(Debug, Clone, PartialEq)]
pub enum ObjType {
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    String(Rc<ObjString>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjString {
    length: usize,
    chars: Vec<u8>,
    hash: u32,
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
