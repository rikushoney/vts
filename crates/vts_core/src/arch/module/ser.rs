use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::arch::{component::ser::ComponentsSerializer, Module};

impl Serialize for Module {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serializer = serializer.serialize_struct("Module", 2)?;

        let name = &self.strings[self.name];
        serializer.serialize_field("name", name)?;

        let components_serializer = ComponentsSerializer::new(self, &self.component_db);
        serializer.serialize_field("components", &components_serializer)?;

        serializer.end()
    }
}
