use std::{
    env, fs,
    io::{stdin, stdout, Write},
    path::Path,
};

use vm::{VirtualMachine, VM};

mod chunk;
mod compiler;
mod debug;
mod object;
mod scanner;
mod utils;
mod value;
mod vm;

fn main() {
    let mut vm = VirtualMachine::init_vm();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        repl(&mut vm);
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

fn repl(vm: &mut VirtualMachine) {
    let mut input = String::new();
    while prompt(&mut input) {
        benchmark!(vm.interpret(&input));
    }
    vm.free_vm();
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
