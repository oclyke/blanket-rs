mod dependency;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

use crate::resource::Root;

pub use dependency::Dependency;

pub trait Build: std::fmt::Debug + std::fmt::Display {
    fn as_any(&self) -> &dyn std::any::Any;
    fn equals(&self, other: Rc<RefCell<dyn Build>>) -> bool;
    fn register(
        self,
        builder: &mut Builder,
    ) -> Result<(Option<PathBuf>, Dependency, Vec<Dependency>), Box<dyn std::error::Error>>;
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

pub struct DependencyGenerator {
    id: u64,
}

impl DependencyGenerator {
    pub fn new() -> Self {
        Self { id: 0 }
    }

    pub fn next(&mut self, data: Rc<RefCell<dyn Build>>) -> Dependency {
        let id = self.id;
        self.id += 1;
        Dependency::new(id, data)
    }
}
