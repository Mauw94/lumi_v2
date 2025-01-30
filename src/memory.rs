use std::rc::Rc;

use crate::value::Obj;

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

// pub fn grow_array<T: std::default::Default>(
//     pointer: Option<Box<[T]>>,
//     old_count: usize,
//     new_count: usize,
// ) -> Box<[T]> {
//     let mut vec = pointer.map(|b| b.into_vec()).unwrap_or_else(Vec::new);
//     vec.resize_with(new_count, Default::default);
//     vec.into_boxed_slice()
// }

// pub fn free_array<T>(pointer: Option<Box<[T]>>, old_count: usize) {
//     if old_count > 0 {
//         drop(pointer);
//     }
// }

// pub fn reallocate<T: std::default::Default>(
//     pointer: Option<Box<[T]>>,
//     old_size: usize,
//     new_size: usize,
// ) -> Option<Box<[T]>> {
//     if new_size == 0 {
//         None
//     } else {
//         Some(grow_array(pointer, old_size, new_size))
//     }
// }
