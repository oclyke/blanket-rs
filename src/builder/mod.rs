mod node;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

pub use node::Node;

#[derive(Debug)]
pub enum Registration {
    Virtual(),
    Concrete(PathBuf),
}

/// A resource that can be built.
pub trait Generate: std::fmt::Debug {
    /// Returns a reference to the resource as `dyn Any`.
    /// Must be implemented for a concrete type as a default implementation
    /// suffers from type erasure.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns true if the resource is equal to the other resource.
    /// Used to determine if the resource has already been registered.
    /// Generally this should return false unless `other` can be downcast to
    /// `Self`.
    fn equals(&self, other: Rc<RefCell<dyn Generate>>) -> bool;

    /// Returns the id of the resource, if it has one.
    /// Used to detect status of resource registration.
    fn id(&self) -> Option<u64>;

    /// Registers the resource with the builder.
    /// Responsibilities:
    /// - Set the id of the resource.
    /// - Require any dependencies of the resource.
    fn register(&mut self, id: u64) -> Result<Registration, Box<dyn std::error::Error>>;

    /// Returns registered nodes of the dependencies.
    fn dependencies(
        &mut self,
        builder: &mut Builder,
    ) -> Result<Vec<Node>, Box<dyn std::error::Error>>;

    /// Generates the resource.
    /// This function will be called after the `generate` method of all the resources
    /// upon which this resource depends have been called.
    fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct Builder {
    dependency_graph: AcyclicDependencyGraph<Node>,
    nodes: HashMap<u64, Node>,
    next_id: u64,
    roots: Vec<Node>,
    output: HashMap<PathBuf, Node>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            dependency_graph: AcyclicDependencyGraph::new(),
            nodes: HashMap::new(),
            next_id: 0,
            roots: vec![],
            output: HashMap::new(),
        }
    }

    pub fn init(self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn require<T: Generate + 'static>(
        &mut self,
        resource: T,
    ) -> Result<Node, Box<dyn std::error::Error>> {
        let node = self.next(Rc::new(RefCell::new(resource)))?;
        self.require_node(node)
    }

    pub fn require_ref<T: Generate + 'static>(
        &mut self,
        resource: Rc<RefCell<T>>,
    ) -> Result<Node, Box<dyn std::error::Error>> {
        let node = self.next(resource)?;
        self.require_node(node)
    }

    pub fn require_node(&mut self, node: Node) -> Result<Node, Box<dyn std::error::Error>> {
        self.nodes.insert(node.id, node.clone());
        self.roots.push(node.clone());
        for dependency in node.clone().dependencies {
            self.dependency_graph
                .depend_on(node.clone(), dependency.clone())
                .unwrap();
            self.require_node(dependency.clone())?;
        }
        Ok(node)
    }

    pub fn generate(self) -> Result<(), Box<dyn std::error::Error>> {
        // perform a topological sort on the dependency graph
        let layers = self
            .dependency_graph
            .get_forward_dependency_topological_layers();

        // generate the site
        for layer in &layers {
            for node in layer {
                node.resource().borrow_mut().generate()?;
            }
        }

        Ok(())
    }

    pub fn output(&self) -> HashMap<PathBuf, Node> {
        self.output.clone()
    }

    pub fn nodes(&self) -> HashMap<u64, Node> {
        self.nodes.clone()
    }

    pub fn roots(&self) -> Vec<Node> {
        self.roots.clone()
    }

    pub fn dependency_graph(&self) -> AcyclicDependencyGraph<Node> {
        self.dependency_graph.clone()
    }

    fn next(
        &mut self,
        resource: Rc<RefCell<dyn Generate>>,
    ) -> Result<Node, Box<dyn std::error::Error>> {
        let optional_id = resource.borrow().id();
        let node = match optional_id {
            Some(id) => {
                let existing = self.nodes.get(&id);
                match existing {
                    Some(node) => node.clone(),
                    None => {
                        let message =
                            format!("Node with id {} expected in nodes but not found", id);
                        return Err(message.into());
                    }
                }
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                let registration = resource.borrow_mut().register(id)?;

                // check for existing node
                let existing = match registration {
                    Registration::Virtual() => None,
                    Registration::Concrete(ref path) => match self.output.get(path) {
                        Some(node) => {
                            let existing = node.resource.borrow();
                            println!("existing: {:?}", existing);
                            println!("resource: {:?}", resource.clone());
                            if !existing.equals(resource.clone()) {
                                println!("path: {:?}", path);
                                println!("existing: {:?}", existing);
                                return Err("output already exists with different data".into());
                            }
                            Some(node.clone())
                        }
                        None => None,
                    },
                };
                if existing.is_some() {
                    return Ok(existing.unwrap());
                }

                // create new node
                let dependencies = resource.borrow_mut().dependencies(self)?;
                let node = Node::new(id, resource.clone(), dependencies);
                match registration {
                    Registration::Virtual() => {}
                    Registration::Concrete(ref path) => {
                        self.output
                            .insert(path.clone(), Node::new(id, resource.clone(), vec![]));
                    }
                };
                node
            }
        };
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[derive(Debug)]
    struct Mock {
        id: Option<u64>,
        path: Option<PathBuf>,
        equals: bool,
        content: Option<String>,
        shared: Option<Rc<RefCell<Mock>>>,
    }

    #[derive(Clone)]
    struct MockBuilder {
        path: Option<PathBuf>,
        equals: bool,
        content: Option<String>,
        shared: Option<Rc<RefCell<Mock>>>,
    }

    impl MockBuilder {
        fn new() -> Self {
            Self {
                path: None,
                equals: false,
                content: None,
                shared: None,
            }
        }
        fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
            self.path = Some(path.as_ref().to_path_buf());
            self
        }
        fn equals(mut self, equals: bool) -> Self {
            self.equals = equals;
            self
        }
        fn content(mut self, content: String) -> Self {
            self.content = Some(content);
            self
        }
        fn shared(mut self, shared: Rc<RefCell<Mock>>) -> Self {
            self.shared = Some(shared);
            self
        }
        fn build(self) -> Mock {
            Mock {
                id: None,
                path: self.path,
                equals: self.equals,
                content: self.content,
                shared: self.shared,
            }
        }
    }

    impl PartialEq for Mock {
        fn eq(&self, other: &Self) -> bool {
            self.equals
        }
    }

    impl Generate for Mock {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn equals(&self, other: Rc<RefCell<dyn Generate>>) -> bool {
            let other = other.borrow();
            let any = other.as_any();
            match any.downcast_ref::<Self>() {
                Some(other) => self == other,
                None => false,
            }
        }
        fn id(&self) -> Option<u64> {
            self.id
        }
        fn register(&mut self, id: u64) -> Result<Registration, Box<dyn std::error::Error>> {
            self.id = Some(id);
            let registration = match self.path.clone() {
                Some(path) => Registration::Concrete(PathBuf::from(path)),
                None => Registration::Virtual(),
            };
            Ok(registration)
        }
        fn dependencies(
            &mut self,
            builder: &mut Builder,
        ) -> Result<Vec<Node>, Box<dyn std::error::Error>> {
            let mut dependencies = vec![];
            if let Some(inner) = self.shared.as_ref() {
                let node = builder.require_ref(inner.clone())?;
                dependencies.push(node);
            }
            Ok(dependencies)
        }
        fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    mod test_mock {
        use std::path::PathBuf;

        #[test]
        fn test_mock_path() {
            const PATH: &str = "/path/to/resource";

            // path defaults to None
            let mock = super::MockBuilder::new().build();
            assert_eq!(mock.path, None);

            // path can be set
            let mock = super::MockBuilder::new().path(PATH).build();
            assert_eq!(mock.path, Some(PathBuf::from(PATH)));
        }

        #[test]
        fn test_mock_equals() {
            // equals defaults to false
            let mock = super::MockBuilder::new().build();
            assert_eq!(mock.equals, false);

            // equals can be set
            let mock = super::MockBuilder::new().equals(true).build();
            assert_eq!(mock.equals, true);
        }
    }

    mod test_builder {
        use super::*;

        #[test]
        fn test_new() {
            let builder = Builder::new();
            assert_eq!(builder.next_id, 0);
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.nodes.len(), 0);
        }

        #[test]
        fn test_make_dependency() {
            let mut builder = Builder::new();
            let mocker = MockBuilder::new();

            // first dependency
            let mock = mocker.clone().build();
            let dependency = builder.require(mock).unwrap();
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.nodes.len(), 1);
            assert_eq!(dependency.id, 0);

            // second dependency
            let mock = mocker.clone().build();
            let dependency = builder.require(mock).unwrap();
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.nodes.len(), 2);
            assert_eq!(dependency.id, 1);
        }

        fn test_require_unique_resources_identical_paths_collide() {
            let mut builder = Builder::new();
            const REGISTRATION_PATH: &str = "identical";

            // add the first mock dependency
            let mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            let result = builder.require(mock);
            assert!(matches!(result, Ok(_)));

            // add the unique mock dependency
            // this should fail because the output path is already registered
            // and the unique mock is configured to indicate that it is not
            // equal to the existing mock
            let unique_mock = MockBuilder::new()
                .path(REGISTRATION_PATH)
                .equals(false)
                .build();
            let result = builder.require(unique_mock);
            assert!(matches!(result, Err(_)));
        }

        #[test]
        fn test_require_identical_resources_identical_paths_ok() {
            let mut builder = Builder::new();
            const REGISTRATION_PATH: &str = "identical";

            // add the first mock dependency
            let mock = MockBuilder::new()
                .path(REGISTRATION_PATH)
                .equals(true)
                .build();
            let result = builder.require(mock);
            assert!(matches!(result, Ok(_)));

            // add the identical mock dependency
            // this should succeed despite being registered at the same output
            // path because the identical mock is configured to indicate that
            // it is equal to the existing mock
            let unique_mock = MockBuilder::new()
                .path(REGISTRATION_PATH)
                .equals(true)
                .build();
            let result = builder.require(unique_mock);
            assert!(matches!(result, Ok(_)));
        }

        #[test]
        fn test_require_unique_paths_ok() {
            let mut builder = Builder::new();
            const REGISTRATION_PATH_1: &str = "path1";
            const REGISTRATION_PATH_2: &str = "path2";
            assert_ne!(REGISTRATION_PATH_1, REGISTRATION_PATH_2);

            // add the first mock dependency
            let mock = MockBuilder::new().path(REGISTRATION_PATH_1).build();
            let result = builder.require(mock);
            assert!(matches!(result, Ok(_)));

            // add the identical mock dependency
            // this should succeed because the output path is different,
            // regardless of the equality of the mocks
            let identical_mock = MockBuilder::new().path(REGISTRATION_PATH_2).build();
            let result = builder.require(identical_mock);
            assert!(matches!(result, Ok(_)));
        }

        #[test]
        fn test_common_resource() {
            let mut builder = Builder::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();
            let common = Rc::new(RefCell::new(common));
            assert!(builder.nodes.is_empty());

            // a resource that depends on the common resource
            let dependent = mocker.clone().shared(common.clone()).build();
            let result = builder.require(dependent);
            assert!(result.is_ok());
            let node = result.unwrap();
            assert_eq!(node.id, 0);
            assert_eq!(builder.nodes.len(), 2);
            assert_eq!(common.borrow().id, Some(1));

            // the common resource should not be duplicated when it is required by another resource
            let dependent = mocker.clone().shared(common.clone()).build();
            let result = builder.require(dependent);
            assert!(result.is_ok());
            let node = result.unwrap();
            assert_eq!(node.id, 2);
            assert_eq!(builder.nodes.len(), 3);
            assert_eq!(common.borrow().id, Some(1));
        }
    }
}
