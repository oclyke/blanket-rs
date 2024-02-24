use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::Generate;

/// Node identifying a resource in the graph.
#[derive(Clone)]
pub struct Node {
    pub id: u64,
    pub resource: Rc<RefCell<dyn Generate>>,
    pub dependencies: Vec<Node>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{id: {:?}, resource: {:?}}}", self.id, self.resource,)
    }
}

impl Node {
    pub fn new(id: u64, resource: Rc<RefCell<dyn Generate>>, dependencies: Vec<Node>) -> Self {
        Self {
            id,
            resource,
            dependencies,
        }
    }

    pub fn resource(&self) -> Rc<RefCell<dyn Generate>> {
        self.resource.clone()
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Node {}
