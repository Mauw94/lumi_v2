use chunk::{Chunk, OpCode, Write};
use debug::disassemble_chunk;
use value::Value;

mod chunk;
mod debug;
mod memory;
mod value;

fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constants(Value::Number(1.2));
    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);

    chunk.write_chunk(OpCode::Return as u8, 123);

    disassemble_chunk(chunk, "test chunk");
}
