use crate::value::Value;

// TODO: we add a max capacity here?
// if (table->count + 1 > table->capacity * TABLE_MAX_LOAD) {
//     int capacity = GROW_CAPACITY(table->capacity);
//     adjustCapacity(table, capacity);
// }
#[derive(Debug, Clone)]
pub struct Table {
    count: usize,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    key: u32,
    value: Value,
}

impl Table {
    pub fn init() -> Self {
        Self {
            count: 0,
            entries: Vec::new(),
        }
    }

    pub fn free(&mut self) {
        self.count = 0;
        self.entries = Vec::new();
    }

    pub fn set(&mut self, key: u32, value: Value) -> bool {
        // println!("setting string: {:?}, {:?}", key, value);
        let entry = self.find_entry(key);
        let is_new_key = entry.is_none();
        if is_new_key {
            self.count += 1;
        }

        if is_new_key {
            self.entries.push(Entry { key, value });
        } else {
            let e = self.entries.iter_mut().find(|e| e.key == key).unwrap();
            e.key = key;
            e.value = value;
        }

        is_new_key
    }

    pub fn get(&self, key: u32) -> Option<&Value> {
        if self.count == 0 {
            return None;
        }

        let entry = self.find_entry(key);
        if entry.is_some() {
            return Some(&entry.unwrap().value);
        }

        None
    }

    pub fn delete(&mut self, key: u32) -> bool {
        if self.count == 0 {
            return false;
        }

        let entry = self.find_entry(key);
        if entry.is_some() {
            let index = self.entries.iter().position(|e| e.key == key).unwrap();
            self.entries.remove(index);
        }

        false
    }

    fn find_entry(&self, key: u32) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key == key)
    }
}
