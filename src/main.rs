use std::{
    env, fs,
    io::{stdin, stdout, Write},
    path::Path,
};

use sysinfo::System;
use vm::VM;

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
    let mut vm = VM::init_vm();
    if args.len() <= 1 {
        repl(&mut vm, &sysinfo);
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

fn repl(vm: &mut VM, _sysinfo: &System) {
    let mut input = String::new();
    while prompt(&mut input) {
        let input_ref: &'static str = Box::leak(input.clone().into_boxed_str());
        benchmark!(vm.interpret(input_ref.trim_end()));

        #[cfg(feature = "bench")]
        if let Some(proc) = _sysinfo.process(sysinfo::get_current_pid().unwrap()) {
            println!("Memory usage: {} bytes", proc.memory());
        } else {
            println!("Failed to get memory usage");
        }
    }
    vm.free_vm();
}

fn run_code(code: &str) {
    let mut vm = VM::init_vm();
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

// #[cfg(test)]
// mod test {

//     use crate::{
//         lnum::{LInt, LNum},
//         object::{Obj, ObjString},
//         value::Value,
//         vm::{InterpretResult, VM},
//     };

//     #[test]
//     fn binary_op_add() {
//         let code: &str = "print 1 + 1;\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Number(LNum::Int(LInt::Small(2))))
//         );
//     }

//     #[test]
//     fn binary_op_minus() {
//         let code: &str = "print 7 - 1;\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Number(LNum::Int(LInt::Small(6))))
//         );
//     }

//     #[test]
//     fn binary_op_divide() {
//         let code: &str = "print 12 / 3;\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Number(LNum::Int(LInt::Small(4))))
//         );
//     }

//     #[test]
//     fn binary_op_multiply() {
//         let code: &str = "print 3 * 7;\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Number(LNum::Int(LInt::Small(21))))
//         );
//     }

//     #[test]
//     fn equals_int() {
//         let code: &str = "print 3 + 7 == 10;\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Bool(true))
//         );
//     }

//     #[test]
//     fn print_string() {
//         let code: &str = "print \"abc\";\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Object(Box::new(Obj::String(ObjString::new(
//                 "abc".as_bytes(),
//                 "abc".as_bytes().len()
//             )))))
//         );
//     }

//     #[test]
//     fn concat_strings() {
//         let code: &str = "print \"a\" + \"b\";\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Object(Box::new(Obj::String(ObjString::new(
//                 "ab".as_bytes(),
//                 "ab".as_bytes().len()
//             )))))
//         );
//     }

//     #[test]
//     fn equals_string() {
//         let code: &str = "print \"test\" + \"a\" == \"testa\";\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Bool(true))
//         );
//     }

//     #[test]
//     fn not_equals_string() {
//         let code: &str = "print \"test\" + \"abc\" == \"ahjskd\";\n";
//         let mut vm = VM::init_vm();
//         assert_eq!(
//             vm.interpret(&code),
//             InterpretResult::InterpretOk(Value::Bool(false))
//         );
//     }
// }
