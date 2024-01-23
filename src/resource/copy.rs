use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{
    builder::{Build, Builder, Dependency},
    resource::Directory,
};

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

impl std::fmt::Display for CopyFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {{ path: {:?} }}",
            std::any::type_name::<Self>()
                .split("::")
                .last()
                .unwrap_or("UnknownType"),
            self.path
        )
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
    ) -> Result<(Option<PathBuf>, Dependency, Vec<Dependency>), Box<dyn std::error::Error>> {
        let path = self.path.clone();
        let parent = match self.path.parent() {
            Some(parent) => parent,
            None => return Err("path has no parent".into()),
        };
        let dir = builder.require(Directory::new(parent))?;
        let dependency = builder.make_dependency(self)?;
        Ok((Some(path), dependency, vec![dir]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let CopyFile { source, path } = self;
        if source.is_dir() {
            return Err("source is a directory".into());
        }
        std::fs::copy(source, path)?;
        Ok(())
    }
}
