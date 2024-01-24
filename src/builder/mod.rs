mod dependency;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

use crate::resource::Root;

pub use dependency::Dependency;

/// A resource that can be built.
pub trait Build: std::fmt::Debug {
    /// Returns a reference to the resource as `dyn Any`.
    /// Must be implemented for a concrete type as a defualt implementation
    /// suffers from type erasure.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns true if the resource is equal to the other resource.
    /// Used to determine if the resource has already been registered.
    /// Generally this should return false unless `other` can be downcast to
    /// `Self`.
    fn equals(&self, other: Rc<RefCell<dyn Build>>) -> bool;

    /// Registers the resource with the builder.
    /// Responsibilities include:
    /// - providing an optional path at which to place the resource in the output
    /// - providing the dependency which contains the registered resource
    /// - providing a vector of dependencies upon which the registered resource depends
    fn register(
        self,
        builder: &mut Builder,
    ) -> Result<(Option<PathBuf>, Dependency, Vec<Dependency>), Box<dyn std::error::Error>>;

    /// Generates the resource.
    /// This function will be called after the `generate` method of all the resources
    /// upon which this resource depends have been called.
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Builder {
    dependency_generator: DependencyGenerator,
    dependency_graph: AcyclicDependencyGraph<Dependency>,
    output: HashMap<PathBuf, Dependency>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            dependency_generator: DependencyGenerator::new(),
            dependency_graph: AcyclicDependencyGraph::new(),
            output: HashMap::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.require(Root {})?;
        Ok(())
    }

    pub fn make_dependency<T: Build + 'static>(
        &mut self,
        resource: T,
    ) -> Result<Dependency, Box<dyn std::error::Error>> {
        let reference = Rc::new(RefCell::new(resource));
        let dependency = self.dependency_generator.next(reference);
        Ok(dependency)
    }

    pub fn add_dependency(
        &mut self,
        from: Dependency,
        to: Dependency,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dependency_graph.depend_on(from, to)?;
        Ok(())
    }

    pub fn require<T: Build>(
        &mut self,
        resource: T,
    ) -> Result<Dependency, Box<dyn std::error::Error>> {
        let (path, dependent, dependencies) = resource.register(self)?;
        let mut dependency = dependent.clone();

        // if the resource returns an output path check if it already exists
        if let Some(path) = path {
            match self.output.get(&path) {
                Some(existing) => {
                    dependency = existing.clone();
                    if !existing.resource().borrow().equals(dependent.resource()) {
                        println!("path: {:?}", path);
                        println!("existing: {}", existing);
                        println!("dependent: {}", dependent);
                        return Err("output already exists with different data".into());
                    }
                }
                None => {
                    self.output.insert(path, dependent.clone());
                    for dependency in dependencies {
                        self.add_dependency(dependent.clone(), dependency)?;
                    }
                }
            }
        }

        Ok(dependency)
    }

    pub fn generate(self) -> Result<(), Box<dyn std::error::Error>> {
        // perform a topological sort on the dependency graph
        let layers = self
            .dependency_graph
            .get_forward_dependency_topological_layers();

        // show the dependency graph
        if layers.len() > 0 {
            println!("\ndependency graph:");
            for layer in &layers {
                println!("layer:");
                for node in layer {
                    println!("  {}", node);
                }
            }
        } else {
            println!("no dependencies to show");
        }

        // show the output
        if self.output.len() > 0 {
            println!("\noutput:");
            for (path, dependency) in &self.output {
                println!("  {:?} -> {}", path, dependency);
            }
        } else {
            println!("no output to generate");
        }

        // generate the site
        for layer in &layers {
            for node in layer {
                node.resource().borrow_mut().generate()?;
            }
        }

        Ok(())
    }
}

/// Generates dependencies from resources.
/// Identifiers monotonically increase.
struct DependencyGenerator {
    id: u64,
}

impl DependencyGenerator {
    fn new() -> Self {
        Self { id: 0 }
    }

    fn next(&mut self, data: Rc<RefCell<dyn Build>>) -> Dependency {
        let id = self.id;
        self.id += 1;
        Dependency::new(id, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[derive(Debug)]
    struct Mock {
        path: Option<PathBuf>,
        equals: bool,
    }

    #[derive(Clone)]
    struct MockBuilder {
        path: Option<PathBuf>,
        equals: bool,
    }

    impl MockBuilder {
        fn new() -> Self {
            Self {
                path: None,
                equals: false,
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
        fn build(self) -> Mock {
            Mock {
                path: self.path,
                equals: self.equals,
            }
        }
    }

    impl Build for Mock {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn equals(&self, _: Rc<RefCell<dyn Build>>) -> bool {
            self.equals
        }

        fn register(
            self,
            builder: &mut Builder,
        ) -> Result<(Option<PathBuf>, Dependency, Vec<Dependency>), Box<dyn std::error::Error>>
        {
            let path = self.path.clone();
            let dependency = builder.make_dependency(self)?;
            Ok((path, dependency.clone(), vec![]))
        }

        fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            assert_eq!(builder.dependency_generator.id, 0);
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.output.len(), 0);
        }

        #[test]
        fn test_init() {
            let mut builder = Builder::new();
            builder.init().unwrap();
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.output.len(), 1);
            assert!(builder.output.contains_key(&PathBuf::from("/")));
            let root = builder.output.get(&PathBuf::from("/")).unwrap();
            assert!(matches!(root, Dependency { id: 0, resource: _ }));
        }

        #[test]
        fn test_make_dependency() {
            let mut builder = Builder::new();
            let mocker = MockBuilder::new();

            // first dependency
            let mock = mocker.clone().build();
            let dependency = builder.make_dependency(mock).unwrap();
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.output.len(), 0);
            assert_eq!(dependency.id, 0);

            // second dependency
            let mock = mocker.clone().build();
            let dependency = builder.make_dependency(mock).unwrap();
            assert_eq!(builder.dependency_graph.is_empty(), true);
            assert_eq!(builder.output.len(), 0);
            assert_eq!(dependency.id, 1);
        }

        #[test]
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
            println!("{:?}", result);
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
        fn test_require_virtual_resources_ok() {
            let mut builder = Builder::new();
            const REGISTRATION_PATH_1: &str = "path1";
            const REGISTRATION_PATH_2: &str = "path2";
            assert_ne!(REGISTRATION_PATH_1, REGISTRATION_PATH_2);

            // add the first virtual mock resource
            let mock = MockBuilder::new().build();
            let result = builder.require(mock);
            assert!(matches!(result, Ok(_)));

            // add the second virtual mock resource
            // this should succeed because virtual resources are not registered
            // in the output
            let identical_mock = MockBuilder::new().build();
            let result = builder.require(identical_mock);
            assert!(matches!(result, Ok(_)));
        }
    }

    mod test_generator {
        use super::*;

        #[test]
        fn test_initial_conditions() {
            let generator = DependencyGenerator::new();
            assert_eq!(generator.id, 0);
        }

        #[test]
        fn test_next() {
            let mut generator = DependencyGenerator::new();
            let mocker = MockBuilder::new();

            // first dependency
            let mock = mocker.clone().build();
            let dependency = generator.next(Rc::new(RefCell::new(mock)));
            assert_eq!(dependency.id, 0);

            // second dependency
            let mock = mocker.clone().build();
            let dependency = generator.next(Rc::new(RefCell::new(mock)));
            assert_eq!(dependency.id, 1);
        }
    }
}
