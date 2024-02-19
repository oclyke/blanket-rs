use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::Build;

/// Identifiable reference to a resource.
#[derive(Clone)]
pub struct Dependency {
    pub id: u64,
    pub resource: Rc<RefCell<dyn Build>>,
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{id: {}, resource: {:p}}}",
            self.id,
            self.resource,
        )
    }
}

impl Dependency {
    pub fn new(id: u64, resource: Rc<RefCell<dyn Build>>) -> Self {
        Self { id, resource }
    }

    pub fn resource(&self) -> Rc<RefCell<dyn Build>> {
        self.resource.clone()
    }
}

impl Hash for Dependency {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Dependency {}
