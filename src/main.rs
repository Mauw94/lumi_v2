use std::{
    env, fs,
    io::{stdin, stdout, Write},
    path::Path,
};

use sysinfo::System;
use vm::{VirtualMachine, VM};

mod chunk;
mod compiler;
mod core;
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
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        repl(&sysinfo);
    } else {
        let filename = &args[1];
        let input_folder = Path::new("runnables");
        let file_path = input_folder.join(filename);
        match fs::read_to_string(&file_path) {
            Ok(content) => run_code(&content),
            Err(err) => eprintln!("Error reading file: {}", err),
        };
    }
}

fn repl(_sysinfo: &System) {
    let mut input = String::new();
    while prompt(&mut input) {
        let mut vm = VirtualMachine::init_vm(&input);
        benchmark!(vm.interpret());

        #[cfg(feature = "bench")]
        if let Some(proc) = _sysinfo.process(sysinfo::get_current_pid().unwrap()) {
            println!("Memory usage: {} bytes", proc.memory());
        } else {
            println!("Failed to get memory usage");
        }
        vm.free_vm();
    }
}

fn run_code(code: &str) {
    let mut vm = VirtualMachine::init_vm(&code);
    benchmark!(vm.interpret());
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
