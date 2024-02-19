use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{
    builder::{Build, Builder, Dependency, Registration},
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
