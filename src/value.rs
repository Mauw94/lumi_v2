use crate::{
    lnum::LNum,
    object::{Obj, ObjString, ObjType},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FinalValue {
    pub value: Value,
    pub is_final: bool,
}

impl FinalValue {
    pub fn default() -> Self {
        Self {
            value: Value::Nil,
            is_final: false,
        }
    }

    pub fn default_new(value: Value) -> Self {
        Self {
            value,
            is_final: false,
        }
    }

    pub fn new(value: Value, is_final: bool) -> Self {
        Self { value, is_final }
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(LNum),
    Bool(bool),
    Object(Box<Obj>),
    Nil,
}

impl Value {
    pub fn default() -> Self {
        Value::Nil
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let Value::Number(value) = self {
            Some(value.real_val())
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<Box<Obj>> {
        if let Value::Object(obj) = self {
            Some(obj.clone())
        } else {
            None
        }
    }

    pub fn bool_val(value: bool) -> Self {
        Value::Bool(value)
    }

    pub fn nil_val() -> Self {
        Value::Nil
    }

    pub fn number_val(value: f64) -> Self {
        Value::Number(LNum::new(value))
    }

    pub fn obj_val(obj: Obj) -> Self {
        Value::Object(Box::new(obj))
    }

    pub fn obj_type(&self) -> Option<ObjType> {
        match self {
            Value::Object(obj) => match &**obj {
                Obj::String(_) => Some(ObjType::String),
            },
            _ => None,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self.obj_type(), Some(ObjType::String))
    }

    pub fn is_obj_type(&self, object_type: ObjType) -> bool {
        // We can unwrap here because self.is_object confirms that we're dealing with an object.
        self.is_object() && self.obj_type().unwrap() == object_type
    }

    pub fn as_string_obj(&self) -> Option<&ObjString> {
        match self {
            Value::Object(obj) => match &**obj {
                Obj::String(obj_string) => Some(obj_string),
            },
            _ => None,
        }
    }

    pub fn as_c_string(&self) -> Option<&str> {
        self.as_string_obj().map(|s| s.as_str())
    }

    pub fn negate(&self) -> Result<Value, String> {
        return match self {
            Value::Number(n) => Ok(Value::Number(n.negate())),
            _ => Err("Negation is only supported for numbers".to_string()),
        };
    }

    pub fn is_same_type(&self, other: &Value) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug, Clone)]
pub struct ValueArray {
    pub values: Vec<FinalValue>,
}

impl ValueArray {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn write_value(&mut self, value: Value, is_final: bool) {
        self.values.push(FinalValue::new(value, is_final));
    }

    pub fn free(&mut self) {
        self.values.clear();
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "nil"),
            Value::Number(val) => write!(f, "{}", val),
            Value::Object(obj) => match &**obj {
                Obj::String(obj_string) => write!(f, "{}", obj_string.as_str()),
            },
        }
    }
}
