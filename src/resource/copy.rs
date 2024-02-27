use regex::Regex;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use log::warn;

use crate::{
    Generate,
    ResourceRef,
    registration::{
        DelayedRegistration, NonterminalRegistration, Registration,
        TerminalRegistration,
    },
};

use crate::resource::Directory;

#[derive(Clone)]
pub struct Filters {
    exclude: Option<Vec<Regex>>,
    include: Option<Vec<Regex>>,
}

type Filter = Box<dyn Fn(&String) -> bool>;

#[derive(Debug)]
pub struct CopyFile {
    source: PathBuf,
    path: PathBuf,
}

impl CopyFile {
    pub fn new<P: AsRef<Path>>(source: P, path: P) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl PartialEq for CopyFile {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Generate for CopyFile {
    fn register(&self, resource: ResourceRef) -> DelayedRegistration {
        let path = self.path.clone();
        Box::new(move || {
            let parent = match path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => {
                    warn!("path has no parent");
                    return Err("path has no parent".into());
                }
            };
            let directory = Rc::new(RefCell::new(Directory::new(parent)));
            Ok(vec![
                Registration::Nonterminal(NonterminalRegistration::DependUnique(
                    resource.clone(),
                    directory.clone(),
                )),
                Registration::Terminal(TerminalRegistration::Concrete(
                    resource.clone(),
                    path.clone(),
                )),
            ])
        })
    }

    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("copying file: {:?} to {:?}", self.source, self.path);

        let CopyFile { source, path, .. } = self;
        if source.is_dir() {
            return Err("source is a directory".into());
        }
        let mut source = std::fs::File::open(source)?;
        let mut dest = std::fs::File::create(path)?;
        std::io::copy(&mut source, &mut dest)?;
        Ok(())
    }
}

pub struct CopyDir {
    source: PathBuf,
    path: PathBuf,
    filters: Filters,
}

impl CopyDir {
    pub fn new<P: AsRef<Path>>(source: P, path: P, filters: Filters) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            path: path.as_ref().to_path_buf(),
            filters,
        }
    }

    pub fn builder<P: AsRef<Path>>(source: P, path: P) -> CopyDirBuilder {
        CopyDirBuilder::new(source, path)
    }
}

impl Generate for CopyDir {
    fn register(&self, resource: ResourceRef) -> DelayedRegistration {
        let source = self.source.clone();
        let path = self.path.clone();
        let filters = self.filters.clone();

        Box::new(move || {
            let filter = build_filter(filters);
            Ok(walkdir::WalkDir::new(source.clone())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();
                    if !path.is_file() {
                        return None;
                    }
                    let relative = match path.strip_prefix(source.clone()) {
                        Ok(rel) => rel.to_path_buf(),
                        Err(_) => return None,
                    };
                    let relative_str = match relative.to_str() {
                        Some(path_str) => path_str,
                        None => return None,
                    };
                    Some(relative_str.to_string())
                })
                .filter(filter.as_ref())
                .map(|relative| (source.join(&relative), path.join(&relative)))
                .map(|(source, path)| CopyFile::new(source, path))
                .map(|file| Rc::new(RefCell::new(file)))
                .map(|object| {
                    Registration::Nonterminal(NonterminalRegistration::DependUnique(
                        resource.clone(),
                        object,
                    ))
                })
                .collect())
        })
    }

    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct CopyDirBuilder {
    source: PathBuf,
    path: PathBuf,

    include: Option<Vec<Regex>>,
    exclude: Option<Vec<Regex>>,
}

impl CopyDirBuilder {
    pub fn new<P: AsRef<Path>>(source: P, path: P) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            path: path.as_ref().to_path_buf(),
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
        let filters = Filters {
            include: self.include,
            exclude: self.exclude,
        };
        CopyDir::new(self.source, self.path, filters)
    }
}

fn build_filter(filters: Filters) -> Filter {
    match filters {
        // both include and exclude filters are present
        // paths are allowed by default
        // exclude acts as a deny list
        // include acts as an allow list with precedence over exclude
        Filters {
            include: Some(filter_include),
            exclude: Some(filter_exclude),
        } => {
            Box::new(move |path: &String| {
                for item in &filter_include {
                    if item.is_match(path) {
                        return true;
                    }
                }
                //
                for item in &filter_exclude {
                    if item.is_match(path) {
                        return false;
                    }
                }
                true
            })
        }

        // only include filter is present
        // paths are denied by default
        // include acts as an allow list
        Filters {
            include: Some(filter_include),
            exclude: None,
        } => Box::new(move |path: &String| {
            for item in &filter_include {
                if item.is_match(path) {
                    return true;
                }
            }
            false
        }),

        // only exclude filter is present
        // paths are allowed by default
        // exclude acts as a deny list
        Filters {
            include: None,
            exclude: Some(filter_exclude),
        } => Box::new(move |path: &String| {
            for item in &filter_exclude {
                if item.is_match(path) {
                    return false;
                }
            }
            true
        }),

        // no filters are present
        // all paths are allowed
        Filters {
            include: None,
            exclude: None,
        } => Box::new(move |_| true),
    }
}
