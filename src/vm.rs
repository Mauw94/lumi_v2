use crate::compiler::Compiler;
use crate::debug::disassemble_instruction;

use crate::{
    chunk::{Chunk, OpCode},
    value::Value,
};

pub trait VM<'a> {
    fn init_vm() -> Self;
    fn free_vm(&self);
    unsafe fn read_byte(&mut self) -> u8;
    fn read_constant(&mut self) -> Value;
    fn binary_op<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> f64;
    fn reset_stack(&mut self);
    fn run(&mut self) -> InterpretResult;
    fn interpret(&mut self, code: &str) -> InterpretResult;
    fn push(&mut self, value: Value);
    fn pop(&mut self) -> &Value;
}

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

pub struct VirtualMachine<'a> {
    chunk: Option<&'a Chunk>,
    ip: *const u8,
    stack: [Value; STACK_MAX],
    stack_top: usize,
}

impl<'a> VM<'a> for VirtualMachine<'a> {
    fn init_vm() -> Self {
        Self {
            chunk: None,
            ip: std::ptr::null(),
            stack: core::array::from_fn(|_| Value::default()),
            stack_top: 0,
        }
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
    }

    fn free_vm(&self) {
        todo!()
    }

    unsafe fn read_byte(&mut self) -> u8 {
        let b = *self.ip;
        self.ip = self.ip.add(1);
        b
    }

    fn read_constant(&mut self) -> Value {
        let index = unsafe { self.read_byte() } as usize;
        self.chunk.as_ref().unwrap().constants.values[index].clone()
    }

    fn binary_op<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let b = self.pop().clone();
        let a = self.pop().clone();
        if let (Value::Number(b), Value::Number(a)) = (b, a) {
            self.push(Value::Number(op(a, b)));
        } else {
            panic!("Binary operation requires two numeric values on the stack.");
        }
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            #[cfg(feature = "trace_exec")]
            trace_execution(self);

            let instruction = unsafe { self.read_byte() };
            match OpCode::from_u8(instruction) {
                Some(OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                Some(OpCode::Negate) => {
                    let value = self.pop().clone();
                    match value.negate() {
                        Ok(negated_value) => self.push(negated_value),
                        Err(err) => panic!("{}", err),
                    }
                }
                Some(OpCode::Add) => {
                    self.binary_op(|a, b| a + b);
                }
                Some(OpCode::Subtract) => {
                    self.binary_op(|a, b| a - b);
                }
                Some(OpCode::Multiply) => {
                    self.binary_op(|a, b| a * b);
                }
                Some(OpCode::Divide) => {
                    self.binary_op(|a, b| a / b);
                }
                Some(OpCode::Return) => {
                    println!("{}", self.pop());
                    return InterpretResult::InterpretOk;
                }
                _ => return InterpretResult::InterpretCompileError,
            };
        }
    }

    fn interpret(&mut self, code: &str) -> InterpretResult {
        // self.chunk = Some(chunk);
        // self.ip = self.chunk.as_ref().unwrap().code.as_ptr();
        // self.run()

        Compiler::compile(code);

        InterpretResult::InterpretOk
    }

    fn push(&mut self, value: Value) {
        if self.stack_top < STACK_MAX {
            self.stack[self.stack_top] = value;
            self.stack_top += 1;
        } else {
            panic!("Stack overflow.");
        }
    }

    fn pop(&mut self) -> &Value {
        self.stack_top -= 1;
        &self.stack[self.stack_top]
    }
}

#[cfg(feature = "trace_exec")]
fn trace_execution(vm: &VirtualMachine) {
    print!("    ");
    for slot in &vm.stack[0..vm.stack_top] {
        print!("[ ");
        print!("{}", *slot);
        print!(" ]");
    }
    println!();
    disassemble_instruction(
        vm.chunk.as_ref().unwrap(),
        vm.ip as usize - vm.chunk.unwrap().code.as_ptr() as usize,
    );
}
