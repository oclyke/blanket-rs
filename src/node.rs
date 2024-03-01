use crate::traits::Generate;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type NodeId = u64;

#[derive(Clone)]
pub struct Node {
    id: NodeId,
    object: Rc<RefCell<dyn Generate>>,
}

impl Node {
    fn new(id: NodeId, object: Rc<RefCell<dyn Generate>>) -> Self {
        Self { id, object }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn object(&self) -> Rc<RefCell<dyn Generate>> {
        self.object.clone()
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node {{ id: {}, object: {:?} }}",
            self.id,
            self.object.borrow()
        )
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.object.borrow().equals(other.object.clone())
    }
}

impl Eq for Node {}

pub struct Manager {
    next_id: NodeId,
    map: HashMap<NodeId, Rc<RefCell<Node>>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            map: HashMap::new(),
        }
    }

    pub fn add_reference(&mut self, object: Rc<RefCell<dyn Generate>>) -> Rc<RefCell<Node>> {
        let resource = Node::new(self.next_id, object);
        let reference = Rc::new(RefCell::new(resource));
        self.insert(self.next_id, reference.clone());
        self.next_id += 1;
        reference
    }

    pub fn generate(&mut self, id: NodeId) -> Result<(), Box<dyn std::error::Error>> {
        match self.map.get(&id) {
            Some(node) => {
                let node = node.borrow_mut();
                let mut object = node.object.borrow_mut();
                object.generate()
            }
            None => Err("resource not found".into()),
        }
    }

    pub fn get(&self, id: NodeId) -> Option<Rc<RefCell<Node>>> {
        self.map.get(&id).map(|node| node.clone())
    }

    fn insert(&mut self, id: NodeId, resource: Rc<RefCell<Node>>) -> Option<Rc<RefCell<Node>>> {
        self.map.insert(id, resource)
    }
}

/// Tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::registration::Registration;

    #[derive(Debug)]
    struct Mock {}

    impl Generate for Mock {
        fn register(&mut self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
            Ok(vec![])
        }
        fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    mod test_manager {
        use super::*;

        #[test]
        fn test_new() {
            let manager = Manager::new();
            assert_eq!(manager.next_id, 0);
            assert_eq!(manager.map.is_empty(), true);
        }

        #[test]
        fn test_add_reference() {
            let mut manager = Manager::new();
            let _reference = manager.add_reference(Rc::new(RefCell::new(Mock {})));
            assert_eq!(manager.next_id, 1);
            assert_eq!(manager.map.len(), 1);
        }
    }
}
