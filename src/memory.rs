use std::rc::Rc;

use crate::object::Obj;

pub fn grow_capacity(capacity: usize) -> usize {
    if capacity < 8 {
        8
    } else {
        capacity * 2
    }
}

pub fn free_objects<'a>(objects: Box<Vec<&'a Obj>>) {
    objects.iter().for_each(|o| free_object((*o).clone()));
}

fn free_object(object: Obj) {
    match object {
        Obj::String(obj_string) => Rc::try_unwrap(obj_string)
            .unwrap_or_else(|rc| (*rc).clone())
            .free(),
    }
}
