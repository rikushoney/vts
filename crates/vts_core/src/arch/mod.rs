use bevy_ecs::prelude::*;
use ustr::{ustr, Ustr};

pub struct Name(Ustr);

impl Name {
    pub fn new(name: &str) -> Self {
        Self(ustr(name))
    }
}

#[derive(Component)]
pub struct Device {
    pub name: Name,
}
