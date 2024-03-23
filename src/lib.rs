use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub mod generator;
pub mod html;

type Filter = Box<dyn Fn(&String) -> bool>;

pub trait Generate {
    fn generate(&self, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>>;
}

pub enum Registration {
    Terminal {
        path: PathBuf,
        generator: Rc<RefCell<dyn Generate>>,
    },
    NonTerminal(Vec<Registration>),
    Deferred(Rc<RefCell<dyn Register>>),
}

pub trait Register {
    fn register(self) -> Result<Registration, Box<dyn std::error::Error>>;
}

pub struct Generator {
    targets: HashMap<PathBuf, Rc<RefCell<dyn Generate>>>,
    filter: Filter,
}

impl Generator {
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            filter: Box::new(|_| true),
        }
    }

    pub fn builder() -> GeneratorBuilder {
        GeneratorBuilder::new()
    }

    pub fn require<R: Register + 'static>(
        &mut self,
        registrant: R,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registrations = expand_registrations(vec![registrant.register()?]);
        for registration in registrations {
            match registration {
                Registration::Terminal { path, generator } => {
                    if !(self.filter)(&path.to_string_lossy().to_string()) {
                        return Err("Path denied by filter".into());
                    }

                    match self.targets.insert(path, generator) {
                        Some(_) => return Err("Duplicate target".into()),
                        None => {}
                    }
                }
                _ => return Err("Unexpected non-terminal registration after expansion".into()),
            }
        }
        Ok(())
    }

    pub fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
        for (output, generator) in self.targets.iter() {
            generator.borrow().generate(output)?;
        }
        Ok(())
    }

    pub fn targets(&self) -> Vec<PathBuf> {
        self.targets.keys().cloned().collect()
    }
}

fn expand_registrations(registrations: Vec<Registration>) -> Vec<Registration> {
    registrations
        .into_iter()
        .map(|registration| match registration {
            Registration::NonTerminal(registrations) => expand_registrations(registrations),
            Registration::Deferred(_registrant) => {
                unimplemented!("Deferred registration");
                // match registrant.borrow().register() {
                //     Ok(registration) => vec![registration],
                //     Err(_) => vec![],
                // }
            }
            registration => vec![registration],
        })
        .flatten()
        .collect()
}

pub struct GeneratorBuilder {
    allow: Option<Vec<Regex>>,
}

impl GeneratorBuilder {
    pub fn new() -> Self {
        Self { allow: None }
    }
    /// Allow registration paths that match the given patterns.
    pub fn allow(mut self, patterns: Vec<&str>) -> Self {
        let regexes = patterns
            .into_iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect();
        self.allow = Some(regexes);
        self
    }
    /// Build the generator.
    pub fn build(self) -> Generator {
        Generator {
            targets: HashMap::new(),
            filter: Self::build_filter(self.allow),
        }
    }

    fn build_filter(filters_allow: Option<Vec<Regex>>) -> Filter {
        match filters_allow {
            // include filter is present
            // paths are denied by default
            // include acts as an allow list
            Some(filters_allow) => Box::new(move |path: &String| {
                for item in &filters_allow {
                    if item.is_match(path) {
                        return true;
                    }
                }
                false
            }),

            // no filters are present
            // all paths are allowed
            None => Box::new(move |_| true),
        }
    }
}
