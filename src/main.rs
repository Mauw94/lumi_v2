use std::{
    env, fs,
    io::{self, stdin, stdout, Write},
    path::Path,
};

use sysinfo::System;
use vm::{VirtualMachine, VM};

mod chunk;
mod compiler;
mod debug;
mod lnum;
mod object;
mod scanner;
mod utils;
mod value;
mod vm;

fn main() {
    let mut sysinfo = System::new_all();
    sysinfo.refresh_all();
    let mut vm = VirtualMachine::init_vm();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        repl(&mut vm, &sysinfo);
    } else {
        let filename = &args[1];
        let input_folder = Path::new("runnables");
        let file_path = input_folder.join(filename);
        match fs::read_to_string(&file_path) {
            Ok(content) => run_code(&mut vm, &content),
            Err(err) => eprintln!("Error reading file: {}", err),
        };
    }

    vm.free_vm();
}

fn repl(vm: &mut VirtualMachine, _sysinfo: &System) {
    let mut input = String::new();
    while prompt(&mut input) {
        benchmark!(interpret(vm, &input));

        #[cfg(feature = "bench")]
        if let Some(proc) = _sysinfo.process(sysinfo::get_current_pid().unwrap()) {
            println!("Memory usage: {} bytes", proc.memory());
        } else {
            println!("Failed to get memory usage");
        }
    }
    vm.free_vm();
}

fn interpret(vm: &mut VirtualMachine, code: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match vm.interpret(code) {
        vm::InterpretResult::InterpretOk => writeln!(handle, "{}", "").unwrap(),
        vm::InterpretResult::InterpretCompileError => {
            writeln!(handle, "{}", "[COMPILE_ERROR]").unwrap()
        }
        vm::InterpretResult::InterpretRuntimeError => {
            writeln!(handle, "{}", "[RUNTIME_ERROR]").unwrap()
        }
    }
}

fn run_code(vm: &mut VirtualMachine, code: &str) {
    benchmark!(vm.interpret(code));
    vm.free_vm();
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

#[macro_export]
macro_rules! benchmark {
    ($expr:expr) => {
        #[cfg(feature = "bench")]
        {
            let start = std::time::Instant::now();
            let result = $expr;
            let duration = start.elapsed();
            println!("Execution time: {}µs", duration.as_micros());
            result
        }
        #[cfg(not(feature = "bench"))]
        {
            $expr
        }
    };
}
