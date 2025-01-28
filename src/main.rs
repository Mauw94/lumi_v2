use chunk::{Chunk, OpCode, Write};
use debug::disassemble_chunk;
use value::Value;
use vm::{VirtualMachine, VM};

mod chunk;
mod debug;
mod memory;
mod value;
mod vm;

fn main() {
    let mut chunk = Chunk::new();
    let mut vm = VirtualMachine::init_vm();

    let mut constant = chunk.add_constants(Value::Number(1.2));
    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);

    chunk.write_chunk(OpCode::Negate as u8, 123);

    chunk.write_chunk(OpCode::Constant as u8, 123);
    constant = chunk.add_constants(Value::Number(5.0));
    chunk.write_chunk(constant as u8, 123);

    chunk.write_chunk(OpCode::Add as u8, 123);

    chunk.write_chunk(OpCode::Return as u8, 123);

    vm.interpret(&chunk);
    // disassemble_chunk(chunk, "test chunk");
}
