use regex::Regex;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{
    builder::{Build, Builder, Dependency, Registration},
    resource::Directory,
};

#[derive(Clone, Debug)]
struct Filters {
    exclude: Vec<Regex>,
    include: Vec<Regex>,
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
        self.path == other.path
    }
}

impl Build for CopyFile {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn equals(&self, other: Rc<RefCell<dyn Build>>) -> bool {
        let other = other.borrow();
        let any = other.as_any();
        match any.downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false,
        }
    }
    fn register(
        self,
        builder: &mut Builder,
    ) -> Result<(Registration, Vec<Dependency>), Box<dyn std::error::Error>> {
        let path = self.path.clone();
        let parent = match self.path.parent() {
            Some(parent) => parent,
            None => return Err("path has no parent".into()),
        };
        let dir = builder.require(Directory::new(parent))?;
        let dependency = builder.make_dependency(self)?;
        Ok((Registration::Concrete(dependency, path), vec![dir]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let CopyFile { source, path } = self;
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
    filter: Filter,
}

impl std::fmt::Debug for CopyDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopyDir")
            .field("source", &self.source)
            .field("path", &self.path)
            .finish()
    }
}

impl CopyDir {
    pub fn new<P: AsRef<Path>>(source: P, path: P, filter: Filter) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            path: path.as_ref().to_path_buf(),
            filter,
        }
    }

    pub fn builder<P: AsRef<Path>>(source: P, path: P) -> CopyDirBuilder {
        CopyDirBuilder::new(source, path)
    }
}

impl PartialEq for CopyDir {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Build for CopyDir {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn equals(&self, other: Rc<RefCell<dyn Build>>) -> bool {
        let other = other.borrow();
        let any = other.as_any();
        match any.downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false,
        }
    }
    fn register(
        self,
        builder: &mut Builder,
    ) -> Result<(Registration, Vec<Dependency>), Box<dyn std::error::Error>> {
        let path = self.path.clone();
        let source = self.source.clone();
        let mut dependencies = vec![];

        if !source.is_dir() {
            return Err("source is not a directory".into());
        }
        // if !path.is_dir() {
        //     return Err("output path is not a directory".into());
        // }
        let paths: Vec<(PathBuf, PathBuf)> = walkdir::WalkDir::new(&source)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if !path.is_file() {
                    return None;
                }
                let relative = match path.strip_prefix(&source) {
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
            .map(|relative| (source.join(&relative), path.join(&relative)))
            .collect();

        for (source, output) in paths {
            let dep = builder.require(CopyFile::new(source, output))?;
            dependencies.push(dep);
        }

        let dependency = builder.make_dependency(self)?;

        Ok((Registration::Virtual(dependency), dependencies))
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
        let filter = build_filter(self.include.clone(), self.exclude.clone());
        CopyDir {
            source: self.source,
            path: self.path,
            filter,
        }
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
        (Some(filters_include), Some(filters_exclude)) => {
            let filters = Filters {
                include: filters_include,
                exclude: filters_exclude,
            };
            Box::new(move |path: &String| {
                for item in &filters.include {
                    if item.is_match(path) {
                        return true;
                    }
                }
                //
                for item in &filters.exclude {
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
        (Some(filters_include), None) => {
            let filters = Filters {
                include: filters_include,
                exclude: Vec::new(),
            };
            Box::new(move |path: &String| {
                for item in &filters.include {
                    if item.is_match(path) {
                        return true;
                    }
                }
                false
            })
        }

        // only exclude filter is present
        // paths are allowed by default
        // exclude acts as a deny list
        (None, Some(filters_exclude)) => {
            let filters = Filters {
                include: Vec::new(),
                exclude: filters_exclude,
            };
            Box::new(move |path: &String| {
                for item in &filters.exclude {
                    if item.is_match(path) {
                        return false;
                    }
                }
                true
            })
        }

        // no filters are present
        // all paths are allowed
        (None, None) => Box::new(move |_| true),
    }
}
