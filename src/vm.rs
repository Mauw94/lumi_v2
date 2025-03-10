use std::io::{self, Write};

use crate::chunk::ChunkWrite;
use crate::compiler::Compiler;
#[cfg(feature = "trace_exec")]
use crate::debug::disassemble_instruction;
use crate::lnum::LNum;
use crate::object::{Obj, ObjString};

use crate::value::FinalValue;
use crate::{chunk::OpCode, value::Value};

#[derive(Debug, PartialEq)]
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

// FIXME: we need a 'shadow' stack of some sorts to be able to evaluate results for testing.
// Our virtual machine.
#[derive(Debug)]
pub struct VM<'a> {
    compiler: Compiler<'a>,
    ip: *const u8,
    stack: [FinalValue; STACK_MAX],
    stack_top: i32,
    objects: Box<Vec<&'a Obj>>,
    had_error: bool,
}

impl<'a> VM<'a> {
    pub fn init_vm() -> Self {
        Self {
            ip: std::ptr::null(),
            stack: core::array::from_fn(|_| FinalValue::default()),
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

        // Get the byte vector as a raw pointer to the memory address.
        self.ip = self.compiler.chunk.code.as_ptr();

        let result = self.run();
        self.compiler.chunk.free();
        self.reset_stack();

        result
    }

    pub fn free_vm(&mut self) {
        self.ip = std::ptr::null();
        self.stack = core::array::from_fn(|_| FinalValue::default());
        self.stack_top = 0;
        self.objects = Box::new(Vec::new());
        self.had_error = false;
        self.compiler.chunk.free();
        self.compiler.globals.free();
        self.compiler.strings.free();
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

        // FIXME: stack is not synced anymore after runtime
        self.reset_stack();
        return InterpretResult::InterpretRuntimeError;
    }

    // Moves the pointer forward 1 byte.
    unsafe fn read_byte(&mut self) -> u8 {
        let b = *self.ip;
        self.ip = self.ip.add(1);
        b
    }

    // Moves the pointer forward 2 bytes.
    unsafe fn read_short(&mut self) -> u16 {
        let high = *self.ip as u16;
        let low = *self.ip.add(1) as u16;
        self.ip = self.ip.add(2);
        (high << 8) | low
    }

    fn read_constant(&mut self) -> FinalValue {
        let index = unsafe { self.read_byte() } as usize;
        self.compiler.chunk.constants.values[index].clone()
    }

    fn binary_op<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> f64,
    {
        if !self.peek(0).value.is_number() || !self.peek(1).value.is_number() {
            self.runtime_error("Operands must be numbers.");
            self.had_error = true;
            return;
        }
        let b = self.pop().value.clone();
        let a = self.pop().value.clone();
        if let (Value::Number(b), Value::Number(a)) = (b, a) {
            let b_val = b.real_val();
            let a_val = a.real_val();
            self.push(FinalValue::default_new(Value::Number(LNum::new(op(
                a_val, b_val,
            )))));
        }
    }

    fn binary_op_bool<F>(&mut self, op: F)
    where
        F: FnOnce(f64, f64) -> bool,
    {
        if !self.peek(0).value.is_number() || !self.peek(1).value.is_number() {
            self.runtime_error("Operands must be numbers.");
            self.had_error = true;
            return;
        }
        let b = self.pop().value.clone();
        let a = self.pop().value.clone();
        if let (Value::Number(b), Value::Number(a)) = (b, a) {
            let b_val = b.real_val();
            let a_val = a.real_val();
            self.push(FinalValue::default_new(Value::Bool(op(a_val, b_val))));
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
                    let fin_val = self.read_constant();
                    let constant = fin_val.value;
                    if constant.is_object() {
                        let obj = constant.as_object().unwrap();
                        self.objects.push(Box::leak(Box::new(obj.clone())));
                    }
                    self.push(FinalValue::new(constant, fin_val.is_final));
                }
                Some(OpCode::Negate) => {
                    if !self.peek(0).value.is_number() {
                        return self.runtime_error("Operand must be a number.");
                    }
                    let value = self.pop().clone();
                    match value.value.negate() {
                        Ok(negated_value) => self.push(FinalValue::default_new(negated_value)),
                        Err(err) => panic!("{}", err),
                    }
                }
                Some(OpCode::Add) => {
                    if self.peek(0).value.is_string() && self.peek(1).value.is_string() {
                        self.concatenate();
                    } else if self.peek(0).value.is_number() && self.peek(1).value.is_number() {
                        let b = self.pop().value.clone();
                        let a = self.pop().value.clone();
                        if let (Value::Number(b), Value::Number(a)) = (b, a) {
                            let b_val = b.real_val();
                            let a_val = a.real_val();
                            self.push(FinalValue::default_new(Value::Number(LNum::new(
                                a_val + b_val,
                            ))));
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
                    let is_falsey = self.is_falsey(value.value);
                    self.push(FinalValue::default_new(Value::Bool(is_falsey)));
                }
                Some(OpCode::Nil) => self.push(FinalValue::default_new(Value::Nil)),
                Some(OpCode::True) => self.push(FinalValue::default_new(Value::Bool(true))),
                Some(OpCode::False) => self.push(FinalValue::default_new(Value::Bool(false))),
                Some(OpCode::Equal) => {
                    let a = self.pop().clone();
                    let b = self.pop().clone();
                    self.push(FinalValue::default_new(Value::Bool(
                        self.values_equal(a.value, b.value),
                    )));
                }
                Some(OpCode::Greater) => self.binary_op_bool(|a, b| a > b),
                Some(OpCode::Less) => self.binary_op_bool(|a, b| a < b),
                Some(OpCode::Return) => {
                    return InterpretResult::InterpretOk;
                }
                Some(OpCode::Print) => {
                    let res = self.pop();
                    println!("{}", res.value);
                }
                Some(OpCode::Pop) => {
                    self.pop();
                }
                Some(OpCode::DefineGlobal) => {
                    let var_name = self.read_constant().value;
                    if let Some(key) = var_name.as_string_obj().clone() {
                        let var_val = self.peek(0).clone();
                        self.compiler.globals.set(key.hash, var_val.value);
                        self.pop();
                        // We pop after the value has been added to the hashtable.
                        // That ensures the VM can still find the variable if a garbage collection.
                        // is triggered right in the middle of adding it to the hash table.
                    } else {
                        return self.runtime_error("Constant is not a string.");
                    }
                }
                Some(OpCode::GetGlobal) => {
                    let fin_value = self.read_constant();
                    let var_name = fin_value.value;
                    if let Some(key) = var_name.as_string_obj().clone() {
                        if let Some(value) = self.compiler.globals.get(key.hash) {
                            self.push(FinalValue::new(value.clone(), fin_value.is_final));
                        } else {
                            return self.runtime_error(
                                format!("Undefined variable {}.", key.as_str()).as_str(),
                            );
                        }
                    } else {
                        return self.runtime_error("Constant is not a string.");
                    }
                }
                Some(OpCode::SetGlobal) => {
                    let final_val = self.read_constant();
                    if final_val.is_final {
                        return self.var_final_error(&final_val);
                    }
                    let var_name = final_val.value;
                    if let Some(key) = var_name.as_string_obj().clone() {
                        let var_val = self.peek(0).clone();
                        if self.compiler.globals.set(key.hash, var_val.value) {
                            self.compiler.globals.delete(key.hash);
                            self.runtime_error(
                                format!("Undefined variable {}.", key.as_str()).as_str(),
                            );
                            return InterpretResult::InterpretRuntimeError;
                        }
                    }
                }
                Some(OpCode::SetLocal) => {
                    let slot = unsafe { self.read_byte() } as usize;
                    let value_to_add_to_stack = self.peek(0).clone();
                    if value_to_add_to_stack.is_final {
                        return self.var_final_error(&value_to_add_to_stack);
                    }
                    self.stack[slot as usize] = self.peek(0).clone();
                }
                Some(OpCode::GetLocal) => {
                    let slot = unsafe { self.read_byte() } as usize;
                    self.push(self.stack[slot].clone());
                }
                Some(OpCode::Jump) => {
                    let offset = unsafe { self.read_short() };
                    self.ip = unsafe { self.ip.add(offset as usize) };
                }
                Some(OpCode::JumpIfFalse) => {
                    let offset = unsafe { self.read_short() };
                    let value = self.peek(0).value.clone();
                    if self.is_falsey(value) {
                        self.ip = unsafe { self.ip.add(offset as usize) };
                    }
                }
                Some(OpCode::Loop) => {
                    let offset = unsafe { self.read_short() };
                    self.ip = unsafe { self.ip.sub(offset as usize) };
                }
                _ => return InterpretResult::InterpretRuntimeError,
            };
        }
    }

    fn push(&mut self, value: FinalValue) {
        if (self.stack_top as usize) < STACK_MAX {
            self.stack[self.stack_top as usize] = value;
            self.stack_top += 1;
        } else {
            panic!("Stack overflow.");
        }
    }

    fn pop(&mut self) -> &FinalValue {
        self.stack_top -= 1;
        &self.stack[self.stack_top as usize]
    }

    fn peek(&mut self, distance: i32) -> &FinalValue {
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

        let b_str = b.value.as_string_obj().unwrap().clone();
        let a_str = a.value.as_string_obj().unwrap().clone();

        let new_val = a_str.to_string() + &b_str.to_string();
        let value = Value::Object(Box::new(Obj::String(ObjString::new(
            new_val.as_bytes(),
            new_val.as_bytes().len(),
        ))));
        self.push(FinalValue::default_new(value));
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

    fn var_final_error(&mut self, final_val: &FinalValue) -> InterpretResult {
        self.runtime_error(
            format!(
                "Variable '{}' is final and cannot be modified.",
                final_val.value
            )
            .as_str(),
        );
        return InterpretResult::InterpretRuntimeError;
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
