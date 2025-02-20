use std::io::{self, Write};

use crate::chunk::ChunkWrite;
use crate::compiler::Compiler;
#[cfg(feature = "trace_exec")]
use crate::debug::disassemble_instruction;
use crate::lnum::LNum;
use crate::object::{Obj, ObjString};

use crate::{chunk::OpCode, value::Value};

#[derive(Debug, PartialEq)]
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

// Our virtual machine.
#[derive(Debug)]
pub struct VM<'a> {
    compiler: Compiler<'a>,
    ip: *const u8,
    stack: [Value; STACK_MAX],
    stack_top: i32,
    objects: Box<Vec<&'a Obj>>,
    had_error: bool,
}

impl<'a> VM<'a> {
    pub fn init_vm() -> Self {
        Self {
            ip: std::ptr::null(),
            stack: core::array::from_fn(|_| Value::default()),
            stack_top: 0,
            objects: Box::new(Vec::new()),
            had_error: false,
            compiler: Compiler::new(),
        }
    }

    pub fn interpret(&mut self, code: &'a str) -> InterpretResult {
        if !self.compiler.compile(code) {
            self.compiler.chunk.free();
            return InterpretResult::InterpretCompileError;
        }
        self.ip = self.compiler.chunk.code.as_ptr();

        let result = self.run();
        self.compiler.chunk.free();
        self.reset_stack();

        result
    }

    pub fn free_vm(&mut self) {
        self.ip = std::ptr::null();
        self.stack = core::array::from_fn(|_| Value::default());
        self.stack_top = 0;
        self.objects = Box::new(Vec::new());
        self.had_error = false;
        self.compiler.chunk.free();
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
    }

    fn runtime_error(&mut self, message: &str) -> InterpretResult {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(handle, "{}", message).unwrap();

        let instruction =
            unsafe { self.ip.offset_from(self.compiler.chunk.code.as_ptr()) as usize - 1 };
        let line = self.compiler.chunk.lines[instruction];
        writeln!(handle, "[line {}] in script", line).unwrap();

        self.reset_stack();
        return InterpretResult::InterpretRuntimeError;
    }

    unsafe fn read_byte(&mut self) -> u8 {
        let b = *self.ip;
        self.ip = self.ip.add(1);
        b
    }

    fn read_constant(&mut self) -> Value {
        let index = unsafe { self.read_byte() } as usize;
        self.compiler.chunk.constants.values[index].clone()
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
            let b_val = b.real_val();
            let a_val = a.real_val();
            self.push(Value::Number(LNum::new(op(a_val, b_val))));
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
            let b_val = b.real_val();
            let a_val = a.real_val();
            self.push(Value::Bool(op(a_val, b_val)));
        }
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            #[cfg(feature = "trace_exec")]
            trace_execution(self);

            if self.had_error {
                self.had_error = false;
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
                        return self.runtime_error("Operand must be a number.");
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
                            let b_val = b.real_val();
                            let a_val = a.real_val();
                            self.push(Value::Number(LNum::new(a_val + b_val)));
                        }
                    } else {
                        return self.runtime_error("Operands must be two numbers or two strings.");
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
                    return InterpretResult::InterpretOk;
                }
                Some(OpCode::Print) => {
                    println!("{}", self.pop());
                }
                Some(OpCode::Pop) => {
                    self.pop();
                }
                Some(OpCode::DefineGlobal) => {
                    let var_name = self.read_constant();
                    if let Some(key) = var_name.as_string_obj().clone() {
                        let var_val = self.pop().clone();
                        self.compiler.globals.set(key.hash, var_val);
                    } else {
                        return self.runtime_error("Constant is not a string.");
                    }
                }
                Some(OpCode::GetGlobal) => {
                    let constant = self.read_constant();
                    if let Some(key) = constant.as_string_obj().clone() {
                        if let Some(value) = self.compiler.globals.get(key.hash) {
                            self.push(value.clone());
                        } else {
                            return self.runtime_error(
                                format!("Undefined variable {}.", key.as_str()).as_str(),
                            );
                        }
                    } else {
                        return self.runtime_error("Constant is not a string.");
                    }
                }
                _ => return InterpretResult::InterpretRuntimeError,
            };
        }
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
        let value = Value::Object(Box::new(Obj::String(ObjString::new(
            new_val.as_bytes(),
            new_val.as_bytes().len(),
        ))));
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
fn trace_execution(vm: &VM) {
    print!("    ");
    for slot in &vm.stack[0..vm.stack_top as usize] {
        print!("[ ");
        print!("{}", *slot);
        print!(" ]");
    }
    println!();
    disassemble_instruction(
        &vm.compiler.chunk,
        vm.ip as usize - vm.compiler.chunk.code.as_ptr() as usize,
    );
}
