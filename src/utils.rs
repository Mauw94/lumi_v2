use std::str;

use crate::lnum::LNum;

pub fn strtod_manual(input: &[u8]) -> Option<LNum> {
    let input_str = str::from_utf8(input).ok()?;

    // Extract the numeric prefix
    let numeric_part: String = input_str.chars().take_while(|c| c.is_digit(10)).collect();

    if numeric_part.is_empty() {
        None
    } else {
        // TODO: can alrdy parse here to LNum?
        let parsed = numeric_part.parse::<f64>().ok()?;
        let lnum = LNum::new(parsed);
        Some(lnum)
    }
}

pub fn hash_str(chars: &[u8], length: usize) -> u32 {
    let mut hash: u32 = 2166136261;
    for i in 0..length {
        hash ^= chars[i] as u32;
        hash = hash.wrapping_mul(16777619);
    }

    hash
}
