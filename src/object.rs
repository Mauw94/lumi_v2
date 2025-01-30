use std::{
    ffi::{c_char, CStr, CString},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ObjType {
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    String(Rc<ObjString>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjString {
    length: usize,
    chars: *mut c_char,
}

impl ObjString {
    pub fn new(bytes: &[u8], length: usize) -> Self {
        let slice = &bytes[..length];
        let s =
            String::from_utf8(slice.to_vec()).expect("Failed to convert to valid UTF-8 string.");

        let c_string = CString::new(s.clone()).expect("CString conversion failed.");
        let length = s.len();
        let chars = c_string.into_raw();

        Self { length, chars }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.chars)
                .to_str()
                .expect("Faield to convert C string to Rust string.")
        }
    }

    pub fn to_string(&self) -> String {
        unsafe { CStr::from_ptr(self.chars).to_string_lossy().into_owned() }
    }

    pub fn free(self) {
        unsafe {
            if !self.chars.is_null() {
                let _ = CString::from_raw(self.chars);
            }
        }
    }
}
