pub mod resource;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

const MAX_RECURSION: usize = 1024;

type ObjectRef = Rc<RefCell<dyn Generate>>;

#[derive(Clone)]
pub struct Resource {
    id: u64,
    object: ObjectRef,
}

impl Hash for Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Resource {}

pub type ResourceRef = Rc<RefCell<Resource>>;

pub type DelayedRegistration = Box<dyn FnOnce() -> Vec<Registration>>;

pub enum Registration {
    /// Nonterminal types
    /// A nonterminal resource will be iteratively expanded into terminal resources.
    // A registration that is not yet ready to be registered.
    Delayed(DelayedRegistration),

    // An Object was created in place and needs to be registered.
    // The Object has not been registered yet.
    Object(ObjectRef),

    /// Terminal types
    // A terminal resource is one which has been fully registered.
    // It has no further dependencies.

    // A dependency on a resource that has already been registered.
    Dependency(ResourceRef, ResourceRef),

    // A Concrete resource must reserve a path in the output directory.
    Concrete(ResourceRef, PathBuf),

    // A Virtual resource is a terminal resource which does not reserve a path in the output directory.
    Virtual(),
}

impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Registration::Delayed(_) => write!(f, "Delayed"),
            Registration::Object(_) => write!(f, "Object"),
            Registration::Dependency(from, to) => write!(
                f,
                "Dependency: {:?} -> {:?}",
                from.borrow().id,
                to.borrow().id
            ),
            Registration::Concrete(resource, path) => {
                write!(f, "Terminal: {:?} at '{:?}'", resource.borrow().id, path)
            }
            Registration::Virtual() => write!(f, "Virtual"),
        }
    }
}

/// A resource that can be built.
pub trait Generate {
    /// Registers the resource with the builder.
    /// Uses a delayed registration which is evaluated lazily at build time.
    fn register(&self, resource: ResourceRef) -> DelayedRegistration;

    /// Generate the resource.
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Generator {
    user_registrations: Vec<DelayedRegistration>,
    resources: HashMap<u64, ResourceRef>,
    next_dependency_id: u64,
    concrete_resources: HashMap<PathBuf, ResourceRef>,
    dependency_graph: AcyclicDependencyGraph<u64>,
}

impl Generator {
    pub fn new() -> Self {
        Self {
            user_registrations: vec![],
            resources: HashMap::new(),
            next_dependency_id: 0,
            concrete_resources: HashMap::new(),
            dependency_graph: AcyclicDependencyGraph::new(),
        }
    }

    /// Create a resource and add it to the list of available resources.
    /// Does not require the resource to be built,
    /// instead it makes it available for other resources to depend on.
    ///
    /// Returns the reference to the new resource.
    pub fn create_resource_from_object<T: Generate + 'static>(&mut self, object: T) -> ResourceRef {
        let object = Rc::new(RefCell::new(object));
        self.create_resource_from_object_reference(object)
    }

    /// Require a resource to be built.
    /// This is the user's primary method of interacting with the builder.
    pub fn require<T: Generate + 'static>(&mut self, object: T) -> ResourceRef {
        // Create the resource.
        let resource = self.create_resource_from_object(object);

        // Add the user's registration.
        let delayed_registration = resource.borrow().object.borrow().register(resource.clone());
        self.user_registrations.push(delayed_registration);
        resource
    }

    pub fn add_dependency(&mut self, resource: ResourceRef, dependency: ResourceRef) {
        self.dependency_graph
            .depend_on(resource.borrow().id, dependency.borrow().id);
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Expand user registrations into terminal registrations.
        let terminals = self.expand_user_registrations()?;

        // Register concrete resources.
        self.register_concrete_resources(terminals)?;

        // Perform a topological sort on the dependency graph.
        // Generate the dependencies in order.
        let layers = self
            .dependency_graph
            .get_forward_dependency_topological_layers();
        for layer in &layers {
            for node in layer {
                let resource = self.resources.get(node).unwrap();
                resource.borrow_mut().object.borrow_mut().generate()?;
            }
        }

        // Generate the concrete resources.
        for (path, resource) in &self.concrete_resources {
            let resource = resource.borrow();
            let path = path.clone();
            resource.object.borrow_mut().generate()?;
        }

        Ok(())
    }

    /// Add an existing object as a resource.
    /// Does not return the resource reference, as the caller should already have it.
    fn create_resource_from_object_reference(&mut self, object: ObjectRef) -> ResourceRef {
        let resource = Resource {
            id: self.next_dependency_id,
            object,
        };
        let reference = Rc::new(RefCell::new(resource));
        self.resources
            .insert(self.next_dependency_id, reference.clone());
        self.next_dependency_id += 1;
        reference
    }

    /// Expands the user registrations into terminal registrations.
    /// Terminal registrations are added to the resources map.
    fn expand_user_registrations(
        &mut self,
    ) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        // Expands a vector of registrations into a vector of terminal registrations recursively.
        fn expand_registrations_recurse(
            recursion_remaining: usize,
            generator: &mut Generator,
            registrations: Vec<Registration>,
        ) -> Vec<Registration> {
            let recursion_remaining = recursion_remaining - 1;
            if recursion_remaining == 0 {
                panic!("Recursion limit reached while expanding registrations.");
            }

            let mut expanded = vec![];
            for registration in registrations {
                println!("registration: {:?}", registration);

                match registration {
                    /// Nonterminal types
                    Registration::Delayed(delayed_registration) => {
                        expanded.extend(expand_registrations_recurse(
                            recursion_remaining,
                            generator,
                            delayed_registration(),
                        ));
                    }
                    Registration::Object(object) => {
                        let resource =
                            generator.create_resource_from_object_reference(object.clone());
                        let delayed_registration = object.borrow().register(resource.clone());
                        expanded.extend(expand_registrations_recurse(
                            recursion_remaining,
                            generator,
                            delayed_registration(),
                        ));
                    }

                    /// Terminal types
                    Registration::Dependency(from, to) => {
                        generator.add_dependency(from, to);
                        // drop the registration
                    }
                    Registration::Concrete(resource, path) => {
                        expanded.push(Registration::Concrete(resource, path));
                    }
                    Registration::Virtual() => {
                        expanded.push(Registration::Virtual());
                    }
                }
            }
            expanded
        }

        // Loop over the user registrations and expand them into an acculator.
        let mut terminal_registrations = vec![];
        while let Some(delayed_registration) = self.user_registrations.pop() {
            let registrations = delayed_registration();
            terminal_registrations.extend(expand_registrations_recurse(
                MAX_RECURSION,
                self,
                registrations,
            ));
        }
        Ok(terminal_registrations)
    }

    fn register_concrete_resources(
        &mut self,
        terminals: Vec<Registration>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for registration in terminals {
            match registration {
                Registration::Virtual() => {
                    // do nothing
                }
                Registration::Concrete(resource, path) => {
                    if self.concrete_resources.contains_key(&path) {
                        return Err(
                            format!("Resource already registered at path: {:?}", path).into()
                        );
                    }

                    println!("registering concrete resource: {:?}", path);

                    self.concrete_resources.insert(path, resource);
                }
                _ => {
                    return Err(
                        "Encountered unexpected nonterminal registration after expansion.".into(),
                    )
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::resource;

    use super::*;
    use std::path::{Path, PathBuf};

    struct Mock {
        path: Option<PathBuf>,
        content: Option<String>,
        shared: Option<(Rc<RefCell<Mock>>, ResourceRef)>,

        generated: Option<String>,
    }

    #[derive(Clone)]
    struct MockBuilder {
        path: Option<PathBuf>,
        content: Option<String>,
        shared: Option<(Rc<RefCell<Mock>>, ResourceRef)>,
    }

    impl MockBuilder {
        fn new() -> Self {
            Self {
                path: None,
                content: None,
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
        fn shared(mut self, shared: (Rc<RefCell<Mock>>, ResourceRef)) -> Self {
            self.shared = Some(shared);
            self
        }
        fn build(self) -> Mock {
            Mock {
                path: self.path,
                content: self.content,
                shared: self.shared,

                generated: None,
            }
        }
    }

    impl Generate for Mock {
        fn register(&self, resource: ResourceRef) -> DelayedRegistration {
            let path = self.path.clone();
            let shared = self.shared.clone();
            Box::new(move || {
                let mut registrations = vec![];

                // Add the shared resource as a dependency.
                match shared {
                    None => {}
                    Some((object, refrence)) => {
                        registrations.push(Registration::Dependency(resource.clone(), refrence));
                    }
                }

                // Add a Virtual or Concrete registration depending on presence of a path.
                let registration = match path {
                    None => Registration::Virtual(),
                    Some(path) => Registration::Concrete(resource.clone(), path),
                };
                registrations.push(registration);

                registrations
            })
        }
        fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            match self.content.clone() {
                None => {}
                Some(content) => {
                    println!("generating mock content: {:?}", content);
                    self.generated = Some(content.clone());
                }
            }

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
        use std::borrow::Borrow;

        use super::*;

        #[test]
        fn test_new() {
            let generator = Generator::new();
            assert_eq!(generator.user_registrations.len(), 0);
            assert_eq!(generator.resources.is_empty(), true);
        }

        #[test]
        fn test_registation() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // first registration
            let mock = mocker.clone().path(PathBuf::from("/some/path")).build();
            generator.require(mock);
            assert_eq!(generator.user_registrations.len(), 1);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // second dependency
            let mock = mocker.clone().path(PathBuf::from("/another/path")).build();
            generator.require(mock);
            assert_eq!(generator.user_registrations.len(), 2);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // expand registrations
            let result = generator.generate();
            assert!(result.is_ok());
            assert_eq!(generator.user_registrations.len(), 0);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.len(), 2);
        }

        #[test]
        fn test_resources_collide() {
            let mut generator = Generator::new();
            const REGISTRATION_PATH: &str = "identical";

            // add the first mock resource
            let mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            generator.require(mock);
            assert_eq!(generator.user_registrations.len(), 1);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // adding a second resource succeeds initially
            let unique_mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            generator.require(unique_mock);
            assert_eq!(generator.user_registrations.len(), 2);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // but when generating the program will panic
            let result = generator.generate();
            assert!(result.is_err());
        }

        #[test]
        fn test_common_resource() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();
            let object = Rc::new(RefCell::new(common));
            let reference = generator.create_resource_from_object_reference(object.clone());
            assert_eq!(generator.user_registrations.len(), 0);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // a resource that depends on the common resource
            let dependent = mocker
                .clone()
                .shared((object.clone(), reference))
                .path("some/concrete/path")
                .build();
            generator.require(dependent);
            assert_eq!(generator.user_registrations.len(), 1);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            let result = generator.generate();
            assert!(result.is_ok());
            assert_eq!(generator.user_registrations.len(), 0);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.len(), 1);
        }
    }
}
