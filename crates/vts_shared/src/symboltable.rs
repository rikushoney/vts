use std::fmt::Debug;

use fnv::FnvHashMap as HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TableKey<I = u32>(I);

// Based on https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html

#[derive(Default)]
pub struct SymbolTable<I = u32> {
    str_keys: HashMap<&'static str, TableKey<I>>,
    table: Vec<&'static str>,
    storage: String,
    archived: Vec<String>,
}

impl<I> SymbolTable<I>
where
    I: Copy + TryFrom<usize> + TryInto<usize>,
    <I as TryFrom<usize>>::Error: Debug,
    <I as TryInto<usize>>::Error: Debug,
{
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        Self {
            str_keys: HashMap::default(),
            table: vec![],
            storage: String::with_capacity(capacity),
            archived: vec![],
        }
    }

    pub fn intern(&mut self, string: &str) -> TableKey<I> {
        if let Some(interned) = self.str_keys.get(string) {
            return *interned;
        }

        let interned = unsafe { self.alloc(string) };
        let key = self
            .table
            .len()
            .try_into()
            .expect("symbol table capacity limit exceeded");
        let key = TableKey(key);
        self.str_keys.insert(interned, key);
        self.table.push(interned);

        key
    }

    pub fn lookup(&self, key: TableKey<I>) -> &str {
        debug_assert!(
            (key.0.try_into().expect("key should fit in usize")) < self.table.len(),
            "key is not in table"
        );
        self.table[key.0.try_into().expect("key should fit in usize")]
    }

    unsafe fn alloc(&mut self, string: &str) -> &'static str {
        use std::cmp;
        use std::mem;

        let capacity = self.storage.capacity();
        if capacity < self.storage.len() + string.len() {
            let new_capacity = cmp::max(capacity, string.len() + 1).next_power_of_two();
            let new_storage = String::with_capacity(new_capacity);
            let old_storage = mem::replace(&mut self.storage, new_storage);
            self.archived.push(old_storage);
        }

        let start = self.storage.len();
        self.storage.push_str(string);
        let interned = &self.storage[start..];

        unsafe { &*(interned as *const str) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let mut table = SymbolTable::<u32>::default();
        let id1 = table.intern("test");
        let id2 = table.intern("test2");
        let id3 = table.intern("test");
        assert_eq!(table.lookup(id1), "test");
        assert_eq!(table.lookup(id2), "test2");
        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }
}
