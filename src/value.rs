use std::ffi::{c_char, CStr, CString};

#[derive(Debug, Clone, PartialEq)]
pub enum ObjType {
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Obj {
    obj_type: ObjType,
}

#[derive(Debug)]
pub struct ObjString {
    obj: Obj,
    length: usize,
    chars: *mut c_char,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64), // TODO: implement like lumi_v1
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
            Some(*value)
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
        Value::Number(value)
    }

    pub fn obj_val(obj: Obj) -> Self {
        Value::Object(Box::new(obj))
    }

    pub fn obj_type(&self) -> Option<ObjType> {
        match self {
            Value::Object(obj) => Some(obj.obj_type.clone()),
            _ => None,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self.obj_type(), Some(ObjType::String))
    }

    pub fn is_obj_type(&self, object_type: ObjType) -> bool {
        // We can unwrap here because self.is_object confirms that we're dealing with an object.
        self.is_object() && self.as_object().unwrap().obj_type == object_type
    }

    pub fn as_string(&self) -> Option<&ObjString> {
        match self {
            Value::Object(obj) => {
                let ObjType::String = obj.obj_type;
                return Some(unsafe { &*(obj.as_ref() as *const Obj as *const ObjString) });
            }
            _ => (),
        }

        None
    }

    pub fn as_c_string(&self) -> Option<&str> {
        self.as_string().map(|s| s.as_str())
    }

    pub fn negate(&self) -> Result<Value, String> {
        return match self {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err("Negation is only supported for numbers".to_string()),
        };
    }

    pub fn is_same_type(&self, other: &Value) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug, Clone)]
pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn write_value(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn free(&mut self) {
        self.values.clear();
    }

    pub fn values_equal(&self, a: &Value, b: &Value) -> bool {
        a == b
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
            Value::Object(obj) => write!(f, "{:?}", obj),
        }
    }
}

impl ObjString {
    pub fn new(s: &str) -> Self {
        let c_string = CString::new(s).expect("CString conversion failed.");
        let length = s.len();
        let chars = c_string.into_raw();

        Self {
            obj: Obj {
                obj_type: ObjType::String,
            },
            length,
            chars,
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.chars)
                .to_str()
                .expect("Faield to convert C string to Rust string.")
        }
    }

    pub fn free(self) {
        unsafe {
            if !self.chars.is_null() {
                let _ = CString::from_raw(self.chars);
            }
        }
    }
}
