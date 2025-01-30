use std::io::{self, Write};

use crate::chunk::ChunkWrite;
use crate::compiler::Compiler;
#[cfg(feature = "trace_exec")]
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
    fn runtime_error(&mut self, message: &str);
    fn run(&mut self) -> InterpretResult;
    fn interpret(&mut self, code: &str) -> InterpretResult;
    fn push(&mut self, value: Value);
    fn pop(&mut self) -> &Value;
    fn peek(&mut self, distance: i32) -> &Value;
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
    stack_top: i32,
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

    fn runtime_error(&mut self, message: &str) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(handle, "{}", message).unwrap();

        let instruction = unsafe {
            self.ip
                .offset_from(self.chunk.as_ref().unwrap().code.as_ptr()) as usize
                - 1
        };
        let line = self.chunk.as_ref().unwrap().lines[instruction];
        writeln!(handle, "[line {}] in script", line).unwrap();

        self.reset_stack();
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
        if !self.peek(0).is_number() || !self.peek(1).is_number() {
            self.runtime_error("Operands must be numbers.");
            return;
        }
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
                    if !self.peek(0).is_number() {
                        self.runtime_error("Operand must be a number.");
                    }
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
                _ => return InterpretResult::InterpretRuntimeError,
            };
        }
    }

    fn interpret(&mut self, code: &str) -> InterpretResult {
        let mut chunk = Chunk::new();
        let mut compiler = Compiler::new(code, &mut chunk);
        if !compiler.compile() {
            chunk.free();
            return InterpretResult::InterpretCompileError;
        }

        self.chunk = Some(Box::leak(Box::new(chunk)));
        self.ip = self.chunk.unwrap().code.as_ptr();

        let result = self.run();
        unsafe {
            Box::from_raw(self.chunk.unwrap() as *const Chunk as *mut Chunk).free();
        }
        self.reset_stack();

        result
    }

    fn push(&mut self, value: Value) {
        if (self.stack_top as usize) < STACK_MAX {
            self.stack[self.stack_top as usize] = value;
            self.stack_top += 1;
        } else {
            panic!("Stack overflow.");
        }
    }

    fn pop(&mut self) -> &Value {
        self.stack_top -= 1;
        &self.stack[self.stack_top as usize]
    }

    fn peek(&mut self, distance: i32) -> &Value {
        if self.stack_top >= 1 + distance {
            &self.stack[(self.stack_top - 1 - distance) as usize]
        } else {
            panic!("Stack is not big enough to peek so far.");
        }
    }
}

#[cfg(feature = "trace_exec")]
fn trace_execution(vm: &VirtualMachine) {
    print!("    ");
    for slot in &vm.stack[0..vm.stack_top as usize] {
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
