#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64), // TODO: implement like lumi_v1
    Bool(bool),
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

    fn as_bool(&self) -> Option<bool> {
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

    pub fn bool_val(value: bool) -> Self {
        Value::Bool(value)
    }

    pub fn nil_val() -> Self {
        Value::Nil
    }

    pub fn number_val(value: f64) -> Self {
        Value::Number(value)
    }

    pub fn negate(&self) -> Result<Value, String> {
        return match self {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err("Negation is only supported for numbers".to_string()),
        };
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
        }
    }
}
