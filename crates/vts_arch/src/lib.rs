use std::sync::Arc;

use fnv::FnvHashMap as HashMap;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DbKey(u32);

pub struct Database<T> {
    lookup_table: Vec<*const T>,
    current: Vec<T>,
    archived: Vec<Vec<T>>,
}

impl<T> Database<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let lookup_table = Vec::new();
        let current = Vec::with_capacity(capacity.next_power_of_two());
        let archived = Vec::new();

        Self {
            lookup_table,
            current,
            archived,
        }
    }

    pub fn entry(&mut self, entity: T) -> DbKey {
        if self.lookup_table.len() == u32::MAX as usize {
            panic!("database limit reached")
        }

        if self.current.len() == self.current.capacity() {
            let new_capacity = self.current.capacity().next_power_of_two();
            let new_storage = Vec::with_capacity(new_capacity);
            let old_storage = std::mem::replace(&mut self.current, new_storage);
            self.archived.push(old_storage);
        }

        self.current.push(entity);

        let id = DbKey(self.lookup_table.len() as u32);
        let ptr = {
            let index = self.current.len() - 1;
            &self.current[index] as *const T
        };
        self.lookup_table.push(ptr);

        id
    }

    /// # Safety
    /// Caller must guarantee that `id` is valid for `self`
    pub unsafe fn lookup(&self, id: DbKey) -> &T {
        // SAFETY: If `id` is valid for `self`, then it will resolve to a
        // pointer that is valid for at least the same duration as `self`.
        &*self.lookup_table[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        #[derive(Debug, PartialEq)]
        struct Foo {
            bar: usize,
        }

        let mut db = Database::<Foo>::with_capacity(1);
        let test1 = db.entry(Foo { bar: 1 });
        let test2 = db.entry(Foo { bar: 2 });
        unsafe {
            assert_eq!(db.lookup(test1), &Foo { bar: 1 });
            assert_eq!(db.lookup(test2), &Foo { bar: 2 });
        }

        let test3 = db.entry(Foo { bar: 3 });
        unsafe {
            assert_eq!(db.lookup(test1), &Foo { bar: 1 });
            assert_eq!(db.lookup(test2), &Foo { bar: 2 });
            assert_eq!(db.lookup(test3), &Foo { bar: 3 });
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Module {
    pub name: Arc<str>,
    pub components: HashMap<Arc<str>, Arc<Component>>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let name = name.into();
        let components = HashMap::default();
        Self { name, components }
    }

    pub fn add_component(&mut self, component: Component) -> Arc<Component> {
        let component = Arc::new(component);
        self.components
            .insert(Arc::clone(&component.name), Arc::clone(&component));
        component
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Component {
    pub name: Arc<str>,
    pub ports: HashMap<Arc<str>, Arc<Port>>,
    pub children: HashMap<Arc<str>, Arc<Component>>,
    pub class: Option<ComponentClass>,
}

impl Component {
    pub fn new(name: &str, class: Option<ComponentClass>) -> Self {
        let name = name.into();
        let ports = HashMap::default();
        let children = HashMap::default();
        Self {
            name,
            ports,
            children,
            class,
        }
    }

    pub fn add_port(&mut self, port: Port) -> Arc<Port> {
        let port = Arc::new(port);
        self.ports.insert(Arc::clone(&port.name), Arc::clone(&port));
        port
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum PortClass {
    #[serde(rename = "CLOCK")]
    Clock,
    #[serde(rename = "LUT_IN")]
    LutIn,
    #[serde(rename = "LUT_OUT")]
    LutOut,
    #[serde(rename = "LATCH_IN")]
    LatchIn,
    #[serde(rename = "LATCH_OUT")]
    LatchOut,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Port {
    pub name: Arc<str>,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl Port {
    pub fn new(name: &str, kind: PortKind, n_pins: usize, class: Option<PortClass>) -> Self {
        let name = name.into();
        Self {
            name,
            kind,
            n_pins,
            class,
        }
    }
}
