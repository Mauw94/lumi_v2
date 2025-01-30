use std::{
    env, fs,
    io::{stdin, stdout, Write},
    path::Path,
};

use vm::{VirtualMachine, VM};

mod chunk;
mod compiler;
mod debug;
mod memory;
mod scanner;
mod utils;
mod value;
mod vm;

fn main() {
    // let mut chunk = Chunk::new();
    // let mut vm = VirtualMachine::init_vm();

    // let mut constant = chunk.add_constants(Value::Number(1.2));
    // chunk.write_chunk(OpCode::Constant as u8, 123);
    // chunk.write_chunk(constant as u8, 123);

    // chunk.write_chunk(OpCode::Negate as u8, 123);

    // chunk.write_chunk(OpCode::Constant as u8, 123);
    // constant = chunk.add_constants(Value::Number(5.0));
    // chunk.write_chunk(constant as u8, 123);

    // chunk.write_chunk(OpCode::Add as u8, 123);

    // chunk.write_chunk(OpCode::Return as u8, 123);

    // vm.interpret(&chunk);
    // disassemble_chunk(chunk, "test chunk");

    let mut vm = VirtualMachine::init_vm();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        repl(&mut vm);
    } else {
        let filename = &args[1];
        let input_folder = Path::new("runnables");
        let file_path = input_folder.join(filename);
        match fs::read_to_string(&file_path) {
            Ok(content) => run_code(&content),
            Err(err) => eprintln!("Error reading file: {}", err),
        };
    }

    vm.free_vm();
}

fn repl(vm: &mut VirtualMachine) {
    let mut input = String::new();
    while prompt(&mut input) {
        vm.interpret(&input);
    }
}

fn run_code(_code: &str) {
    todo!()
}

fn prompt(input: &mut String) -> bool {
    input.clear();
    print!("lumi> ");
    if stdout().flush().is_err() {
        return false;
    }

    match stdin().read_line(input) {
        Ok(_) => true,
        Err(_) => false,
    }
}
