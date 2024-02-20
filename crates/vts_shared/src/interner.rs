use fnv::FnvHashMap as HashMap;

use std::mem::replace;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct StringId(u32);

#[derive(Default)]
pub struct Interner {
    strings: HashMap<&'static str, StringId>,
    table: Vec<&'static str>,
    current: String,
    filled: Vec<String>,
}

impl Interner {
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        Self {
            strings: HashMap::default(),
            table: vec![],
            current: String::with_capacity(capacity),
            filled: vec![],
        }
    }

    pub fn intern(&mut self, string: &str) -> StringId {
        if let Some(interned) = self.strings.get(string) {
            return *interned;
        }

        let interned = unsafe { self.alloc(string) };
        let id = StringId(self.current.len() as u32);
        self.strings.insert(interned, id);
        self.table.push(interned);

        id
    }

    pub fn lookup(&self, id: StringId) -> &str {
        self.table[id.0 as usize]
    }

    unsafe fn alloc(&mut self, string: &str) -> &'static str {
        let capacity = self.current.capacity();
        if capacity < self.current.len() + string.len() {
            let new_capacity = capacity.max(string.len() + 1).next_power_of_two();
            let new_buffer = String::with_capacity(new_capacity);
            let old_buffer = replace(&mut self.current, new_buffer);
            self.filled.push(old_buffer);
        }

        let start = self.current.len();
        self.current.push_str(string);
        let interned = &self.current[start..];

        &*(interned as *const str)
    }
}
