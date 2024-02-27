#![feature(trait_upcasting)]

pub mod resource;
pub mod registration;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use std::any::Any;

use log::{error, trace};
use topologic::AcyclicDependencyGraph;

use registration::{DelayedRegistration, NonterminalRegistration, Registration, TerminalRegistration};

/// The maximum recursion depth for delayed registrations.
const MAX_RECURSION: usize = 1024;

/// An object that can be built.
pub trait Generate: Any {
    /// Registers the object with the builder.
    /// Uses a delayed registration which is evaluated lazily at build time.
    fn register(&self, resource: ResourceRef) -> DelayedRegistration;

    /// Generate the object.
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns true if the object is equal to the other object.
    /// Used to allow output location sharing for compatible objects.
    /// Generally this should return false unless `other` can be downcast to
    /// `Self`.
    fn equals(&self, _other: ObjectRef) -> bool {
        false
    }
}

/// A reference to a `Generate` object.
type ObjectRef = Rc<RefCell<dyn Generate>>;

/// A resource.
/// Wraps an object and provides a unique identifier.
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

/// A reference to a `Resource`.
pub type ResourceRef = Rc<RefCell<Resource>>;

/// A resource generator.
pub struct Generator {
    registrations: Vec<DelayedRegistration>,
    resources: HashMap<u64, ResourceRef>,
    next_dependency_id: u64,
    concrete_resources: HashMap<PathBuf, ResourceRef>,
    dependency_graph: AcyclicDependencyGraph<u64>,
}

impl Generator {
    pub fn new() -> Self {
        trace!("Generator::new()");
        Self {
            registrations: vec![],
            resources: HashMap::new(),
            next_dependency_id: 0,
            concrete_resources: HashMap::new(),
            dependency_graph: AcyclicDependencyGraph::new(),
        }
    }

    /// Require a resource to be built.
    /// This is the user's primary method of interacting with the builder.
    pub fn require<T: Generate + 'static>(&mut self, object: T) -> ResourceRef {
        trace!("Generator::require()");

        // Create the resource.
        let resource = self.create_resource_from_object(object);

        // Add the user's registration.
        let delayed = resource.borrow().object.borrow().register(resource.clone());
        self.registrations.push(delayed);
        resource
    }

    /// Create a resource and add it to the list of available resources.
    /// Does not require the resource to be built,
    /// instead it makes it available for other resources to depend on.
    ///
    /// Returns the reference to the new resource.
    pub fn create_resource_from_object<T: Generate + 'static>(&mut self, object: T) -> ResourceRef {
        trace!("Generator::create_resource_from_object()");
        let object = Rc::new(RefCell::new(object));
        self.create_resource_from_object_reference(object)
    }

    /// Add an existing object as a resource.
    /// Does not return the resource reference, as the caller should already have it.
    pub fn create_resource_from_object_reference(&mut self, object: ObjectRef) -> ResourceRef {
        trace!("Generator::create_resource_from_object_reference()");

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

    pub fn add_dependency(
        &mut self,
        resource: ResourceRef,
        dependency: ResourceRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self
            .dependency_graph
            .depend_on(resource.borrow().id, dependency.borrow().id)
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        trace!("Generator::generate()");

        // Expand user registrations into terminal registrations.
        let mut terminals = vec![];
        while let Some(delayed) = self.registrations.pop() {
            let expanded = self.expand_registrations(delayed()?)?;
            terminals.extend(expanded);
        }

        // Register concrete resources.
        for registration in terminals {
            match registration {
                Registration::Nonterminal(_) => {
                    let message =
                        "Encountered unexpected nonterminal registration after expansion.";
                    error!("{}", message);
                    return Err(message.into());
                }
                Registration::Terminal(terminal) => {
                    match terminal {
                        TerminalRegistration::Virtual(_) => {
                            // do nothing
                        }
                        TerminalRegistration::Concrete(resource, path) => {
                            // register the concrete resource
                            self.add_concrete_resource(resource, path)?;
                        }
                    }
                }
            }
        }

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
        for (_path, resource) in &self.concrete_resources {
            let resource = resource.borrow();
            resource.object.borrow_mut().generate()?;
        }

        Ok(())
    }

    /// Expand user registrations into terminal registrations.
    fn expand_registrations(
        &mut self,
        registrations: Vec<Registration>,
    ) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        // Recursive function to expand the registrations.
        fn expand_registrations_recursive(
            recurse: usize,
            generator: &mut Generator,
            registrations: Vec<Registration>,
        ) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
            if recurse == 0 {
                return Err("Maximum recursion depth reached.".into());
            }
            let recurse = recurse - 1;
            let mut collected = vec![];

            for registration in registrations {
                match registration {
                    Registration::Nonterminal(nonterminal) => match nonterminal {
                        NonterminalRegistration::Delayed(delayed) => {
                            let expanded =
                                expand_registrations_recursive(recurse, generator, delayed()?)?;
                            collected.extend(expanded);
                        }
                        NonterminalRegistration::DependUnique(resource, object) => {
                            let unique = generator.create_resource_from_object_reference(object);
                            let delayed = unique.borrow().object.borrow().register(unique.clone());
                            let expanded =
                                expand_registrations_recursive(recurse, generator, delayed()?)?;
                            generator.add_dependency(resource, unique)?;
                            collected.extend(expanded);
                        }
                        NonterminalRegistration::DependShared(resource, shared) => {
                            generator.add_dependency(resource, shared)?;
                        }
                    },
                    Registration::Terminal(terminal) => {
                        collected.push(Registration::Terminal(terminal));
                    }
                }
            }

            Ok(collected)
        }

        // Expand the registrations.
        let terminals = expand_registrations_recursive(MAX_RECURSION, self, registrations)?;
        Ok(terminals)
    }

    /// Add a concrete resource to the generator.
    /// Returns Ok(()) if the resource is added successfully.
    /// Returns an error if the resource already exists at the path and is not equal to the new resource.
    fn add_concrete_resource(
        &mut self,
        resource: ResourceRef,
        path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.concrete_resources.get(&path) {
            Some(existing) => {
                if !resource
                    .borrow()
                    .object
                    .borrow()
                    .equals(existing.borrow().object.clone())
                {
                    return Err(
                        format!("Concrete resource already exists at path: {:?}", path).into(),
                    );
                }
            }
            None => {}
        }
        self.concrete_resources.insert(path, resource);
        Ok(())
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

        // generated content.
        // after call to generate() Mock.content == Mock.generated.
        generated: Option<String>,

        // a unique resource.
        // represents a resource created in-place by the mock resource.
        unique: Option<Rc<RefCell<Mock>>>,

        // a shared resource and its reference.
        // represents a resource created externally and used by the mock resource.
        shared: Option<(Rc<RefCell<Mock>>, ResourceRef)>,
    }

    #[derive(Clone)]
    struct MockBuilder {
        path: Option<PathBuf>,
        content: Option<String>,
        shared: Option<(Rc<RefCell<Mock>>, ResourceRef)>,
        unique: Option<Rc<RefCell<Mock>>>,
    }

    impl MockBuilder {
        fn new() -> Self {
            Self {
                path: None,
                content: None,
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
        fn unique(mut self, unique: Mock) -> Self {
            self.unique = Some(Rc::new(RefCell::new(unique)));
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
                unique: self.unique,
                shared: self.shared,

                generated: None,
            }
        }
    }

    impl Generate for Mock {
        fn register(&self, resource: ResourceRef) -> DelayedRegistration {
            let path = self.path.clone();
            let unique = self.unique.clone();
            let shared = self.shared.clone();

            let resource = resource.clone();
            Box::new(move || {
                let mut registrations = vec![];

                // Add the unique resource as a dependency.
                match unique {
                    None => {}
                    Some(unique) => {
                        registrations.push(Registration::Nonterminal(
                            NonterminalRegistration::DependUnique(resource.clone(), unique.clone()),
                        ));
                    }
                }

                // Add the shared resource as a dependency.
                match shared {
                    None => {}
                    Some((_object, reference)) => {
                        registrations.push(Registration::Nonterminal(
                            NonterminalRegistration::DependShared(resource.clone(), reference),
                        ));
                    }
                }

                // Add the concrete resource.
                match path {
                    None => {}
                    Some(path) => {
                        registrations.push(Registration::Terminal(TerminalRegistration::Concrete(
                            resource.clone(),
                            path,
                        )));
                    }
                }

                return Ok(registrations);
            })
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
            assert_eq!(generator.registrations.len(), 0);
            assert_eq!(generator.resources.is_empty(), true);
        }

        #[test]
        fn test_registation() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // first registration
            let mock = mocker.clone().path(PathBuf::from("/some/path")).build();
            generator.require(mock);
            assert_eq!(generator.registrations.len(), 1);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // second dependency
            let mock = mocker.clone().path(PathBuf::from("/another/path")).build();
            generator.require(mock);
            assert_eq!(generator.registrations.len(), 2);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // expand registrations
            let result = generator.generate();
            assert!(result.is_ok());
            assert_eq!(generator.registrations.len(), 0);
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
            assert_eq!(generator.registrations.len(), 1);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // adding a second resource succeeds initially
            let unique_mock = MockBuilder::new().path(REGISTRATION_PATH).build();
            generator.require(unique_mock);
            assert_eq!(generator.registrations.len(), 2);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // but when generating the program will panic
            let result = generator.generate();
            assert!(result.is_err());
        }

        #[test]
        fn test_unique_resource() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();
            assert_eq!(generator.registrations.len(), 0);
            assert_eq!(generator.resources.len(), 0);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // a resource that depends on the common resource
            let dependent = mocker
                .clone()
                .unique(common)
                .path("some/concrete/path")
                .build();
            generator.require(dependent);
            assert_eq!(generator.registrations.len(), 1);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            let result = generator.generate();
            assert!(result.is_ok());
            assert_eq!(generator.registrations.len(), 0);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.len(), 1);
        }

        #[test]
        fn test_shared_resource() {
            let mut generator = Generator::new();
            let mocker = MockBuilder::new();

            // a common resource
            let common = mocker.clone().content(String::from("shared")).build();
            let object = Rc::new(RefCell::new(common));
            let reference = generator.create_resource_from_object_reference(object.clone());
            assert_eq!(generator.registrations.len(), 0);
            assert_eq!(generator.resources.len(), 1);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            // a resource that depends on the common resource
            let dependent = mocker
                .clone()
                .shared((object.clone(), reference))
                .path("some/concrete/path")
                .build();
            generator.require(dependent);
            assert_eq!(generator.registrations.len(), 1);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.is_empty(), true);

            let result = generator.generate();
            assert!(result.is_ok());
            assert_eq!(generator.registrations.len(), 0);
            assert_eq!(generator.resources.len(), 2);
            assert_eq!(generator.concrete_resources.len(), 1);
        }
    }
}
