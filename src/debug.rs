use crate::chunk::{Chunk, OpCode};

#[allow(dead_code)]
pub fn disassemble_chunk(chunk: Chunk, chunk_name: &str) {
    println!("== {} == \n", chunk_name);

    println!("{}", chunk.count);
    let mut offset = 0;
    while offset < chunk.count {
        offset += disassemble_instruction(&chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    match OpCode::from_u8(instruction) {
        Some(OpCode::Constant) => constant_instruction("OP_CONSTANT", chunk, offset),
        Some(OpCode::Nil) => simple_instruction("OP_NIL"),
        Some(OpCode::False) => simple_instruction("OP_FALSE"),
        Some(OpCode::Equal) => simple_instruction("OP_EQUAL"),
        Some(OpCode::Greater) => simple_instruction("OP_GREATER"),
        Some(OpCode::Less) => simple_instruction("OP_LESS"),
        Some(OpCode::True) => simple_instruction("OP_TRUE"),
        Some(OpCode::Add) => simple_instruction("OP_ADD"),
        Some(OpCode::Subtract) => simple_instruction("OP_SUBTRACT"),
        Some(OpCode::Multiply) => simple_instruction("OP_MULTIPLY"),
        Some(OpCode::Divide) => simple_instruction("OP_DIVIDE"),
        Some(OpCode::Not) => simple_instruction("OP_NOT"),
        Some(OpCode::Negate) => simple_instruction("OP_NEGATE"),
        Some(OpCode::Return) => simple_instruction("OP_RETURN"),
        Some(OpCode::Print) => simple_instruction("OP_PRINT"),
        Some(OpCode::Pop) => simple_instruction("OP_POP"),
        Some(OpCode::DefineGlobal) => simple_instruction("OP_DEFINE_GLOBAL"),
        None => {
            println!("Unknown opcode {}", instruction);
            offset + 1
        }
    }
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant_index = chunk.code[offset + 1] as usize;
    print!("{:<16} {:4} '", name, constant_index);
    if let Some(value) = chunk.constants.values.get(constant_index) {
        print!("{}", value);
    }
    println!("'");
    2
}

fn simple_instruction(name: &str) -> usize {
    println!("{}", name);
    1
}
