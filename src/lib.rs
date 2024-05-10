mod structure;
// mod cache;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use topologic::AcyclicDependencyGraph;

use cache::Cache;

pub mod cache;
pub mod targets;

/// Analyze
/// Trait for target analysis
pub trait Analyze {
    fn dependencies(&self) -> Vec<PathBuf>;
    fn output(&self) -> PathBuf;
}

/// Generate
/// Trait for target generation
pub trait Generate {
    /// Generate the target and return the path to the generated target
    fn generate(&self);
    // fn generate(&self) -> Result<PathBuf, Box<dyn std::error::Error>>;
}

/// Target
/// Trait for targets which can be added to the build
pub trait Target: Analyze + Generate {}

/// Generator
pub struct Generator {
    structure: structure::Structure,
    graph: AcyclicDependencyGraph<PathBuf>,
    targets: HashMap<PathBuf, Rc<RefCell<dyn Target>>>,
}

impl Generator {
    pub fn new() -> Self {
        let mut structure = structure::Structure::new();
        structure::initialize(&mut structure);

        Self {
            structure,
            graph: AcyclicDependencyGraph::new(),
            targets: HashMap::new(),
        }
    }

    pub fn add_targets(&mut self, targets: Vec<impl Target + 'static>) {
        for target in targets {
            self.add_target(target);
        }
    }

    pub fn add_target(&mut self, target: impl Target + 'static) {
        let target = Rc::new(RefCell::new(target));
        let output = target.borrow().output();
        structure::add_node(&mut self.structure, &output, structure::Node::File);

        self.targets.insert(output.clone(), target.clone());

        let dependencies = target.borrow().dependencies();
        for dependency in dependencies {
            self.graph
                .depend_on(output.to_path_buf(), dependency.to_path_buf())
                .expect("Failed to add dependency");
        }
    }

    pub fn generate<H: Eq + std::fmt::Debug, C: Cache<H>>(
        &mut self,
        mut cache: C,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut rebuilt = HashSet::new();

        // ensure that the build structure exists
        // (in the future perhaps the user can hook in to a visitor pattern over all the structure nodes)
        for (path, node) in &self.structure {
            match node {
                structure::Node::Directory => {
                    std::fs::create_dir_all(&path)?;
                }
                _ => {}
            }
        }

        let layers = self.graph.get_forward_dependency_topological_layers();
        for layer in layers {
            for node in layer {
                match self.targets.get(&node) {
                    Some(target) => {
                        // a rule exists for generating this target
                        // the question is now whether or not it actually needs to be generated

                        // case 1: the target does not exist and must be generated
                        // case 2: the target exists but its dependencies have changed and it must be regenerated
                        // case 3: the target exists and its dependencies have not changed, so it does not need to be regenerated

                        println!("inspecting target: {:?}", node);

                        // at this point all dependencies have been built (or found) and their hashes are in the cache
                        // get the hashes of all the dependencies
                        let mut needs_rebuild = false;
                        let dependencies = target.borrow().dependencies();
                        let mut current_dependency_hashes = HashMap::new();
                        for dependency in &dependencies {
                            let result = cache.get(&dependency);
                            if let Some((hash, _)) = result {
                                current_dependency_hashes.insert(dependency.clone(), hash);
                            }
                        }

                        if !node.exists() {
                            println!("\ttarget does not exist. generating...");
                            needs_rebuild = true;
                        } else {
                            println!("\ttarget exists. checking dependencies for changes...");

                            println!("node: {:?}", node);

                            // check all possible cases for rebuilding
                            // case 1: a change in the generation rule (not implemented - this would require a hash of the rule itself)
                            //   case 1.a: the build system has changed... probably no need to implement this at our scale - this is a Bazel level thing
                            // case 2: a change in the set of dependencies
                            //   case 2.a a new dependency has been added
                            //   case 2.b a dependency has been dropped (this is not necessarily cause for rebuild, but we can simplify by rebuilding in this case too)
                            // case 3: a change in the hash of a dependency
                            // case 4: the target does not have any dependency information in the cache, it must be rebuilt

                            // we are working with two sets of dependencies
                            // a: the cached_dependencies, which were stored in the cache the last time the target was generated
                            // b: the current_dependencies, which is the set of dependencies that the target currently has

                            if let Some((_, cached_dependency_hashes)) = cache.get(&node) {
                                let current_dependencies: HashSet<PathBuf> =
                                    current_dependency_hashes.keys().cloned().collect();
                                let cached_dependencies: HashSet<PathBuf> =
                                    cached_dependency_hashes.keys().cloned().collect();

                                // if the sets are not equal then we need to rebuild
                                if current_dependencies != cached_dependencies {
                                    println!("\t\tdependencies have changed");
                                    needs_rebuild = true;
                                }

                                // if any of the hashes of the dependencies have changed then we need to rebuild
                                for (dependency, hash) in &current_dependency_hashes {
                                    println!("dependency: {:?}", dependency);
                                    let previous_hash = cached_dependency_hashes
                                        .get(dependency)
                                        .expect("dependency not found in cached dependencies");
                                    if hash != previous_hash {
                                        println!("\t\tdependency {:?} has changed", dependency);
                                        // println!("\t\told hash: {:?}", previous_hash);
                                        // println!("\t\tnew hash: {:?}", hash);
                                        needs_rebuild = true;
                                        break;
                                    }
                                }
                            } else {
                                println!("\t\ttarget has no cached dependency information");
                                needs_rebuild = true;
                            }
                        }

                        if needs_rebuild {
                            println!("\t\tregenerating target...");
                            let target = target.borrow();
                            target.generate();
                            let hash = cache.hash(&node);
                            cache.set(&node, &hash, &current_dependency_hashes);
                            rebuilt.insert(node.clone());
                        }
                    }
                    None => {
                        let exists = node.exists();
                        match exists {
                            true => {
                                // cache files which exist but have no rule to generate them
                                let hash = cache.hash(&node);
                                // let hash = blake3::hash(&std::fs::read(&node)?);
                                let deps = HashMap::new();

                                println!("caching: {:?}", node);

                                cache.set(&node, &hash, &deps);
                            }
                            false => {
                                return Err(format!(
                                    "Node {:?} does not exist and has no rule to generate it",
                                    node
                                )
                                .into());
                            }
                        }
                    }
                }
            }
        }

        println!("rebuilt: {:?}", rebuilt);

        Ok(())
    }
}
