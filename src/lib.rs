#![feature(trait_upcasting)]

pub mod resource;

mod node;
mod registration;
mod root;
mod traits;

pub use registration::Registration;
pub use traits::Generate;
pub use node::{Node, NodeId};

use node::Manager as NodeManager;
use root::Root;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

/// The maximum recursion depth for delayed registrations.
const MAX_RECURSION: usize = 1024;

/// A resource generator.
pub struct Generator {
    nodes: NodeManager,
    dependency_graph: AcyclicDependencyGraph<NodeId>,
    paths: HashMap<PathBuf, NodeId>,
}

impl Generator {
    pub fn new() -> Self {
        // Create the resource manager and add the root resource.
        let mut nodes = NodeManager::new();
        let root = Root {};
        let root = Rc::new(RefCell::new(root));
        nodes.add_reference(root);

        Self {
            nodes,
            dependency_graph: AcyclicDependencyGraph::new(),
            paths: HashMap::new(),
        }
    }

    /// Require an object to be built.
    /// Primary method of interacting with the builder.
    pub fn require<T: Generate + 'static>(
        &mut self,
        object: T,
    ) -> Result<Rc<RefCell<Node>>, Box<dyn std::error::Error>> {
        let reference = Rc::new(RefCell::new(object));
        self.require_reference(reference)
    }

    /// Require an object to be built via reference.
    /// Used internally to handle recursive registration.
    pub fn require_reference(
        &mut self,
        reference: Rc<RefCell<dyn Generate>>,
    ) -> Result<Rc<RefCell<Node>>, Box<dyn std::error::Error>> {
        self.require_reference_recursive(0, reference)
    }

    fn require_reference_recursive(
        &mut self,
        depth: usize,
        reference: Rc<RefCell<dyn Generate>>,
    ) -> Result<Rc<RefCell<Node>>, Box<dyn std::error::Error>> {
        if depth > MAX_RECURSION {
            return Err("maximum recursion depth exceeded".into());
        }
        let depth = depth + 1;

        let node = self.nodes.add_reference(reference.clone());
        let id = node.borrow().id();
        let registrations = reference.borrow().register()?;

        for registration in registrations {
            match registration {
                Registration::RequireRoot() => {
                    self.add_dependency(id, 0)?;
                }
                Registration::RequireUnique(unique) => {
                    let unique_node = self.require_reference_recursive(depth, unique)?;
                    self.add_dependency(id, unique_node.borrow().id())?;
                }
                Registration::RequireShared(shared) => {
                    let object = shared.borrow().object();
                    let shared_node = self.require_reference_recursive(depth, object)?;
                    self.add_dependency(id, shared_node.borrow().id())?;
                }
                Registration::ReservePath(path) => match self.paths.insert(path.clone(), id) {
                    None => {}
                    Some(existing_id) => {
                        let existing = match self.nodes.get(existing_id) {
                            Some(existing) => existing,
                            None => {
                                return Err("resource not found".into());
                            }
                        };
                        if node != existing {
                            println!("path: {:?}", path.clone());
                            println!("node: {:?}", node);
                            println!("existing: {:?}", existing);
                            return Err("output already exists with different data".into());
                        }
                    }
                },
                Registration::PrecedeUnique(unique) => {
                    let unique_node = self.require_reference_recursive(depth, unique)?;
                    self.add_dependency(unique_node.borrow().id(), id)?;
                }
                Registration::PrecedeShared(shared) => {
                    let object = shared.borrow().object();
                    let shared_node = self.require_reference_recursive(depth, object)?;
                    self.add_dependency(shared_node.borrow().id(), id)?;
                }
            }
        }

        Ok(node)
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Perform a topological sort on the dependency graph.
        // Generate the dependencies in order.
        let layers = self
            .dependency_graph
            .get_forward_dependency_topological_layers();
        for layer in &layers {
            for id in layer {
                self.nodes.generate(*id)?;
            }
        }

        Ok(())
    }

    /// Indicate a dependency relationship between two nodes.
    /// The `from` node depends on the `to` node.
    fn add_dependency(
        &mut self,
        from: NodeId,
        to: NodeId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.dependency_graph.depend_on(from, to) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// Tests.
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct Mock {
        // path to the resource.
        // if Some the resource is concrete.
        path: Option<PathBuf>,

        // content of the resource.
        content: Option<String>,

        // equivalency.
        // if true the resource will report as equivalent to any other resource.
        equivalent: bool,

        // generated content.
        // after call to generate() Mock.content == Mock.generated.
        generated: Option<String>,

        // a unique resource.
        // represents a resource created in-place by the mock resource.
        unique: Option<Rc<RefCell<Mock>>>,

        // a shared resource and its reference.
        // represents a resource created externally and used by the mock resource.
        shared: Option<(Rc<RefCell<Mock>>, Rc<RefCell<Node>>)>,
    }

    #[derive(Clone)]
    struct MockBuilder {
        path: Option<PathBuf>,
        content: Option<String>,
        equivalent: bool,
        shared: Option<(Rc<RefCell<Mock>>, Rc<RefCell<Node>>)>,
        unique: Option<Rc<RefCell<Mock>>>,
    }

    impl std::fmt::Debug for Mock {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let shared = match &self.shared {
                None => None,
                Some(_) => Some("shared"),
            };
            f.debug_struct("Mock")
                .field("path", &self.path)
                .field("content", &self.content)
                .field("unique", &self.unique)
                .field("shared", &shared)
                .field("generated", &self.generated)
                .finish()
        }
    }

    impl MockBuilder {
        fn new() -> Self {
            Self {
                path: None,
                content: None,
                equivalent: false,
                unique: None,
                shared: None,
            }
        }
        fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
            self.path = Some(path.as_ref().to_path_buf());
            self
        }
        fn content(mut self, content: String) -> Self {
            self.content = Some(content);
            self
        }
        fn equivalent(mut self) -> Self {
            self.equivalent = true;
            self
        }
        fn unique(mut self, unique: Mock) -> Self {
            self.unique = Some(Rc::new(RefCell::new(unique)));
            self
        }
        fn shared(mut self, shared: (Rc<RefCell<Mock>>, Rc<RefCell<Node>>)) -> Self {
            self.shared = Some(shared);
            self
        }
        fn build(self) -> Mock {
            Mock {
                path: self.path,
                content: self.content,
                equivalent: self.equivalent,
                unique: self.unique,
                shared: self.shared,

                generated: None,
            }
        }
    }

    impl Generate for Mock {
        fn equals(&self, _other: Rc<RefCell<dyn Generate>>) -> bool {
            self.equivalent
        }

        fn register(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
            let mut registrations = vec![];

            // Add the unique resource as a dependency.
            match self.unique {
                None => {}
                Some(ref unique) => {
                    registrations.push(Registration::RequireUnique(unique.clone()));
                }
            }

            // Add the shared resource as a dependency.
            match self.shared {
                None => {}
                Some((ref _object, ref reference)) => {
                    registrations.push(Registration::RequireShared(reference.clone()));
                }
            }

            // Reserve the path.
            match self.path.clone() {
                None => {}
                Some(path) => {
                    registrations.push(Registration::ReservePath(path));
                }
            }

            Ok(registrations)
        }

        fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            // copy content to generated
            match self.content.clone() {
                None => {}
                Some(content) => {
                    self.generated = Some(content.clone());
                }
            }

            // check content generated in the shared resource
            match self.shared.clone() {
                None => {}
                Some((object, _)) => match object.borrow().generated.clone() {
                    None => {}
                    Some(content) => {
                        println!("shared content: {:?}", content);
                    }
                },
            }

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
    }

    mod test_generator {
        use super::*;

        #[test]
        fn test_new() {
            let generator = Generator::new();
            assert_eq!(generator.dependency_graph.is_empty(), true);
        }

        #[test]
        fn test_registation() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // first registration
            let mock = mocker.clone().path(PathBuf::from("/some/path")).build();
            let result = generator.require(mock);
            assert!(result.is_ok());

            // second dependency
            let mock = mocker.clone().path(PathBuf::from("/another/path")).build();
            let result = generator.require(mock);
            assert!(result.is_ok());

            // expand registrations
            let result = generator.generate();
            assert!(result.is_ok());
        }

        #[test]
        fn test_resources_collide() {
            let mut generator = Generator::new();
            const REGISTRATION_PATH: &str = "identical";

            // add the first mock resource
            let mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            let result = generator.require(mock);
            assert!(result.is_ok());

            // adding a second resource fails
            let unique_mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            let result = generator.require(unique_mock);
            assert!(result.is_err());
        }

        #[test]
        fn test_equivalent_resources_no_collide() {
            let mut generator = Generator::new();
            const REGISTRATION_PATH: &str = "identical";

            // add the first mock resource
            let mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            let result = generator.require(mock);
            assert!(result.is_ok());

            // adding a second resource succeeds when the resource is equivalent
            let unique_mock = MockBuilder::new()
                .path(REGISTRATION_PATH)
                .equivalent()
                .build();
            let result = generator.require(unique_mock);
            assert!(result.is_ok());
        }

        #[test]
        fn test_unique_resource() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();

            // a resource that depends on the common resource
            let dependent = mocker
                .clone()
                .unique(common)
                .path("some/concrete/path")
                .build();
            let result = generator.require(dependent);
            assert!(result.is_ok());

            let result = generator.generate();
            assert!(result.is_ok());
        }

        #[test]
        fn test_shared_resource() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();
            let object = Rc::new(RefCell::new(common));
            let reference = generator.require_reference(object.clone());
            assert!(reference.is_ok());
            let reference = reference.unwrap();

            // a resource that depends on the common resource
            let dependent = mocker
                .clone()
                .shared((object.clone(), reference))
                .path("some/concrete/path")
                .build();
            let result = generator.require(dependent);
            assert!(result.is_ok());

            let result = generator.generate();
            assert!(result.is_ok());
        }
    }
}
