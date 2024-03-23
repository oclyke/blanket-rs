use crate::{Generate, Register, Registration};

use regex::Regex;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

type Filter = Box<dyn Fn(&String) -> bool>;

pub struct CopyFile {
    source: PathBuf,
    destination: PathBuf,
}

impl CopyFile {
    pub fn new(source: &PathBuf, destination: &PathBuf) -> Self {
        Self {
            source: source.clone(),
            destination: destination.clone(),
        }
    }
}

impl Generate for CopyFile {
    fn generate(&self, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let dir = output.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::copy(&self.source, output)?;
        Ok(())
    }
}

impl Register for CopyFile {
    fn register(self) -> Result<Registration, Box<dyn std::error::Error>> {
        Ok(Registration::Terminal {
            path: self.destination.clone(),
            generator: Rc::new(RefCell::new(self)),
        })
    }
}

pub struct CopyDir {
    source: PathBuf,
    destination: PathBuf,
    filter: Filter,
}

impl CopyDir {
    pub fn new(source: &PathBuf, destination: &PathBuf, filter: Filter) -> Self {
        Self {
            source: source.clone(),
            destination: destination.clone(),
            filter,
        }
    }
    pub fn builder(source: &PathBuf, destination: &PathBuf) -> CopyDirBuilder {
        CopyDirBuilder::new(source, destination)
    }
}

impl Register for CopyDir {
    fn register(self) -> Result<Registration, Box<dyn std::error::Error>> {
        Ok(Registration::NonTerminal(
            walkdir::WalkDir::new(self.source.clone())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let source_path = e.path();
                    if !source_path.is_file() {
                        return None;
                    }
                    let relative = match source_path.strip_prefix(self.source.clone()) {
                        Ok(rel) => rel.to_path_buf(),
                        Err(_) => return None,
                    };
                    let relative_str = match relative.to_str() {
                        Some(path_str) => path_str,
                        None => return None,
                    };
                    Some(relative_str.to_string())
                })
                .filter(self.filter.as_ref())
                .map(|relative| {
                    let source = self.source.clone().join(relative.clone());
                    let destination = self.destination.join(&relative);
                    Registration::Terminal {
                        path: destination.clone(),
                        generator: Rc::new(RefCell::new(CopyFile {
                            source,
                            destination,
                        })),
                    }
                })
                .collect(),
        ))
    }
}

pub struct CopyDirBuilder {
    source: PathBuf,
    destination: PathBuf,
    include: Option<Vec<Regex>>,
    exclude: Option<Vec<Regex>>,
}

impl CopyDirBuilder {
    pub fn new(source: &PathBuf, destination: &PathBuf) -> Self {
        Self {
            source: source.clone(),
            destination: destination.clone(),
            include: None,
            exclude: None,
        }
    }
    pub fn include(mut self, patterns: Vec<&str>) -> Self {
        let regexes = patterns
            .into_iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect();
        self.include = Some(regexes);
        self
    }
    pub fn exclude(mut self, patterns: Vec<&str>) -> Self {
        let regexes = patterns
            .into_iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect();
        self.exclude = Some(regexes);
        self
    }
    pub fn build(self) -> CopyDir {
        CopyDir {
            source: self.source,
            destination: self.destination,
            filter: Self::build_filter(self.include, self.exclude),
        }
    }

    fn build_filter(
        filters_include: Option<Vec<Regex>>,
        filters_exclude: Option<Vec<Regex>>,
    ) -> Filter {
        match (filters_include, filters_exclude) {
            // both include and exclude filters are present
            // paths are allowed by default
            // exclude acts as a deny list
            // include acts as an allow list with precedence over exclude
            (Some(filters_include), Some(filters_exclude)) => Box::new(move |path: &String| {
                for item in &filters_include {
                    if item.is_match(path) {
                        return true;
                    }
                }
                for item in &filters_exclude {
                    if item.is_match(path) {
                        return false;
                    }
                }
                true
            }),

            // only include filter is present
            // paths are denied by default
            // include acts as an allow list
            (Some(filters_include), None) => Box::new(move |path: &String| {
                for item in &filters_include {
                    if item.is_match(path) {
                        return true;
                    }
                }
                false
            }),

            // only exclude filter is present
            // paths are allowed by default
            // exclude acts as a deny list
            (None, Some(filters_exclude)) => Box::new(move |path: &String| {
                for item in &filters_exclude {
                    if item.is_match(path) {
                        return false;
                    }
                }
                true
            }),

            // no filters are present
            // all paths are allowed
            (None, None) => Box::new(move |_| true),
        }
    }
}
