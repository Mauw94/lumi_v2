use std::io::{self, Write};
use std::rc::Rc;

use crate::chunk::ChunkWrite;
use crate::compiler::Compiler;
#[cfg(feature = "trace_exec")]
use crate::debug::disassemble_instruction;
use crate::memory::free_objects;
use crate::object::{Obj, ObjString};

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
    fn binary_op_bool<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> bool;
    fn reset_stack(&mut self);
    fn runtime_error(&mut self, message: &str);
    fn run(&mut self) -> InterpretResult;
    fn interpret(&mut self, code: &str) -> InterpretResult;
    fn push(&mut self, value: Value);
    fn pop(&mut self) -> &Value;
    fn peek(&mut self, distance: i32) -> &Value;
    fn is_falsey(&mut self, value: Value) -> bool;
    fn concatenate(&mut self);
    fn values_equal(&self, a: Value, b: Value) -> bool;
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
    objects: Box<Vec<&'a Obj>>,
    had_error: bool,
}

impl<'a> VM<'a> for VirtualMachine<'a> {
    fn init_vm() -> Self {
        Self {
            chunk: None,
            ip: std::ptr::null(),
            stack: core::array::from_fn(|_| Value::default()),
            stack_top: 0,
            objects: Box::new(Vec::new()),
            had_error: false,
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
        free_objects(self.objects.clone());
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
            self.had_error = true;
            return;
        }
        let b = self.pop().clone();
        let a = self.pop().clone();
        if let (Value::Number(b), Value::Number(a)) = (b, a) {
            self.push(Value::Number(op(a, b)));
        }
    }

    fn binary_op_bool<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> bool,
    {
        if !self.peek(0).is_number() || !self.peek(1).is_number() {
            self.runtime_error("Operands must be numbers.");
            self.had_error = true;
            return;
        }
        let b = self.pop().clone();
        let a = self.pop().clone();
        if let (Value::Number(b), Value::Number(a)) = (b, a) {
            self.push(Value::Bool(op(a, b)));
        }
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            #[cfg(feature = "trace_exec")]
            trace_execution(self);

            if self.had_error {
                return InterpretResult::InterpretRuntimeError;
            }

            let instruction = unsafe { self.read_byte() };
            match OpCode::from_u8(instruction) {
                Some(OpCode::Constant) => {
                    let constant = self.read_constant();
                    if constant.is_object() {
                        let obj = constant.as_object().unwrap();
                        self.objects.push(Box::leak(Box::new(obj.clone())));
                    }
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
                    if self.peek(0).is_string() && self.peek(1).is_string() {
                        self.concatenate();
                    } else if self.peek(0).is_number() && self.peek(1).is_number() {
                        let b = self.pop().clone();
                        let a = self.pop().clone();
                        if let (Value::Number(b), Value::Number(a)) = (b, a) {
                            self.push(Value::Number(a + b));
                        }
                    } else {
                        self.runtime_error("Operands must be two numbers or two strings.");
                    }
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
                Some(OpCode::Not) => {
                    let value = self.pop().clone();
                    let is_falsey = self.is_falsey(value);
                    self.push(Value::Bool(is_falsey));
                }
                Some(OpCode::Nil) => self.push(Value::Nil),
                Some(OpCode::True) => self.push(Value::Bool(true)),
                Some(OpCode::False) => self.push(Value::Bool(false)),
                Some(OpCode::Equal) => {
                    let a = self.pop().clone();
                    let b = self.pop().clone();
                    self.push(Value::Bool(self.values_equal(a, b)));
                }
                Some(OpCode::Greater) => self.binary_op_bool(|a, b| a > b),
                Some(OpCode::Less) => self.binary_op_bool(|a, b| a < b),
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

    fn is_falsey(&mut self, value: Value) -> bool {
        value.is_nil() || (value.is_bool() && !value.as_bool().unwrap())
    }

    fn concatenate(&mut self) {
        let b = self.pop().clone();
        let a = self.pop().clone();

        let b_str = b.as_string_obj().unwrap().clone();
        let a_str = a.as_string_obj().unwrap().clone();

        let new_val = a_str.to_string() + &b_str.to_string();
        let value = Value::Object(Box::new(Obj::String(Rc::new(ObjString::new(
            new_val.as_bytes(),
            new_val.as_bytes().len(),
        )))));
        self.push(value);
    }

    fn values_equal(&self, a: Value, b: Value) -> bool {
        if !a.is_same_type(&b) {
            return false;
        }
        match a {
            Value::Number(_) => a == b,
            Value::Bool(_) => a == b,
            Value::Object(ref obj) => match &**obj {
                Obj::String(_) => a.as_c_string() == b.as_c_string(),
            },
            Value::Nil => a == b,
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
