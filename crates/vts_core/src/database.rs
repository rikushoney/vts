use std::iter;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice;

use crate::OpaqueKey;

pub trait DbKey: OpaqueKey {}

impl DbKey for u8 {}
impl DbKey for u16 {}
impl DbKey for u32 {}
impl DbKey for u64 {}

#[derive(Clone, Debug, PartialEq)]
pub struct Database<T, I = u32> {
    lookup_table: Vec<*const T>,
    current: Vec<T>,
    archived: Vec<Vec<T>>,
    _unused: PhantomData<I>,
}

const DEFAULT_DATABASE_CAPACITY: usize = 16;

impl<T, I: DbKey> Default for Database<T, I> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_DATABASE_CAPACITY)
    }
}

impl<T, I: DbKey> Database<T, I> {
    pub fn with_capacity(capacity: usize) -> Self {
        let lookup_table = Vec::new();
        let current = Vec::with_capacity(capacity.next_power_of_two());
        let archived = Vec::new();

        Self {
            lookup_table,
            current,
            archived,
            _unused: PhantomData,
        }
    }

    pub fn entry(&mut self, entity: T) -> I {
        use std::mem;

        assert!(self.lookup_table.len() < I::max_index());

        if self.current.len() == self.current.capacity() {
            let new_capacity = self.current.capacity().next_power_of_two();
            let new_storage = Vec::with_capacity(new_capacity);
            let old_storage = mem::replace(&mut self.current, new_storage);
            self.archived.push(old_storage);
        }

        self.current.push(entity);

        let id = I::from_index(self.lookup_table.len());
        let ptr = {
            let index = self.current.len() - 1;
            &self.current[index] as *const T
        };
        self.lookup_table.push(ptr);

        id
    }

    fn lookup(&self, id: I) -> &T {
        let index = id.as_index();
        assert!(index < self.lookup_table.len());
        let ptr = self.lookup_table[index];

        // SAFETY: `ptr` is valid for the same lifetime as `self`.
        unsafe { &*ptr }
    }

    pub fn lookup_mut(&mut self, id: I) -> &mut T {
        let index = id.as_index();
        assert!(index < self.lookup_table.len());
        let ptr = self.lookup_table[index] as *mut T;

        // SAFETY: we have an exclusive reference to `self` (which implies
        // exclusive access to the value behind `ptr`) and `ptr` is valid for
        // the same lifetime as `self`.
        unsafe { &mut *ptr }
    }

    pub fn len(&self) -> usize {
        self.lookup_table.len()
    }

    pub fn iter(&self) -> DatabaseIter<'_, T, I> {
        DatabaseIter {
            iter: iter::zip(self.keys(), self.values()),
            _database: self,
        }
    }

    pub fn iter_mut(&mut self) -> DatabaseIterMut<'_, T, I> {
        DatabaseIterMut {
            iter: iter::zip(self.keys(), self.values_mut()),
            _database: self,
        }
    }

    pub fn keys(&self) -> DatabaseKeys<I> {
        DatabaseKeys {
            index: I::from_index(0),
        }
    }

    pub fn values(&self) -> DatabaseValues<'_, T, I> {
        DatabaseValues {
            iter: self.lookup_table.iter(),
            _database: self,
        }
    }

    pub fn values_mut(&mut self) -> DatabaseValuesMut<'_, T, I> {
        DatabaseValuesMut {
            iter: self.lookup_table.iter(),
            _database: self,
        }
    }
}

impl<T, I: DbKey> Index<I> for Database<T, I> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.lookup(index)
    }
}

impl<T, I: DbKey> IndexMut<I> for Database<T, I> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.lookup_mut(index)
    }
}

pub struct DatabaseIter<'a, T, I> {
    iter: iter::Zip<DatabaseKeys<I>, DatabaseValues<'a, T, I>>,
    _database: &'a Database<T, I>,
}

impl<'a, T, I: DbKey> Iterator for DatabaseIter<'a, T, I> {
    type Item = (I, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct DatabaseIterMut<'a, T, I> {
    iter: iter::Zip<DatabaseKeys<I>, DatabaseValuesMut<'a, T, I>>,
    _database: &'a mut Database<T, I>,
}

impl<'a, T, I: DbKey> Iterator for DatabaseIterMut<'a, T, I> {
    type Item = (I, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct DatabaseKeys<I> {
    index: I,
}

impl<I: DbKey> Iterator for DatabaseKeys<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index.increment();
        if index.as_index() <= I::max_index() {
            Some(index)
        } else {
            None
        }
    }
}

pub struct DatabaseValues<'a, T, I> {
    iter: slice::Iter<'a, *const T>,
    _database: &'a Database<T, I>,
}

impl<'a, T, I> Iterator for DatabaseValues<'a, T, I> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&ptr) = self.iter.next() {
            // SAFETY: pointers in the database are valid for the lifetime of the database.
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct DatabaseValuesMut<'a, T, I> {
    iter: slice::Iter<'a, *const T>,
    _database: &'a mut Database<T, I>,
}

impl<'a, T, I> Iterator for DatabaseValuesMut<'a, T, I> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&ptr) = self.iter.next() {
            // SAFETY: same as `DatabaseValues`
            // we also have an exclusive reference to the database (via `_database`) which
            // means it is safe to mutate values
            Some(unsafe { &mut *ptr })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database() {
        #[derive(Debug, PartialEq)]
        struct Foo {
            bar: usize,
        }

        let mut db = Database::<Foo>::with_capacity(1);
        let test1 = db.entry(Foo { bar: 1 });
        let test2 = db.entry(Foo { bar: 2 });
        assert_eq!(db[test1], Foo { bar: 1 });
        assert_eq!(db[test2], Foo { bar: 2 });

        let test3 = db.entry(Foo { bar: 3 });
        assert_eq!(db[test1], Foo { bar: 1 });
        assert_eq!(db[test2], Foo { bar: 2 });
        assert_eq!(db[test3], Foo { bar: 3 });
    }
}
