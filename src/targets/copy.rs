use crate::{Analyze, Generate, Target};
use regex::Regex;
use std::path::PathBuf;

type Filter = Box<dyn Fn(&String) -> bool>;

pub struct CopyFile {
    source: PathBuf,
    destination: PathBuf,
}

impl CopyFile {
    pub fn new(source: PathBuf, destination: PathBuf) -> Self {
        Self {
            source,
            destination,
        }
    }
}

impl Analyze for CopyFile {
    fn dependencies(&self) -> Vec<PathBuf> {
        vec![self.source.clone()]
    }
    fn output(&self) -> PathBuf {
        self.destination.clone()
    }
}

impl Generate for CopyFile {
    fn generate(&self) {
        std::fs::copy(&self.source, &self.destination).expect("Failed to copy file");
    }
}

impl Target for CopyFile {}

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

    pub fn targets(self) -> Vec<CopyFile> {
        let sources = walkdir::WalkDir::new(&self.source)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.path().to_string_lossy().to_string())
            .filter(|path| (self.filter)(path))
            .map(|path| PathBuf::from(path))
            .collect::<Vec<PathBuf>>();

        let mut targets = vec![];
        for source in sources {
            let relative = source.strip_prefix(&self.source).unwrap();
            let destination = self.destination.join(relative);
            let target = CopyFile::new(source, destination);
            targets.push(target);
        }
        return targets;
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
