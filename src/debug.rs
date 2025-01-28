use crate::chunk::{Chunk, OpCode};

pub fn disassemble_chunk(chunk: Chunk, chunk_name: &str) {
    println!("== {} == \n", chunk_name);

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
        Some(OpCode::Nil) => simple_instruction("OP_NIL", offset),
        Some(OpCode::False) => simple_instruction("OP_FALSE", offset),
        Some(OpCode::True) => simple_instruction("OP_TRUE", offset),
        Some(OpCode::Add) => simple_instruction("OP_ADD", offset),
        Some(OpCode::Subtract) => simple_instruction("OP_SUBTRACT", offset),
        Some(OpCode::Multiply) => simple_instruction("OP_MULTIPLY", offset),
        Some(OpCode::Divide) => simple_instruction("OP_DIVIDE", offset),
        Some(OpCode::Not) => simple_instruction("OP_NOT", offset),
        Some(OpCode::Negate) => simple_instruction("OP_NEGATE", offset),
        Some(OpCode::Return) => simple_instruction("OP_RETURN", offset),
        Some(_) | None => {
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
    offset + 2
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}
