use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::builder::{Build, Builder, Dependency};
use crate::resource::Root;

#[derive(Debug)]
pub struct Directory {
    path: PathBuf,
}

impl Directory {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl std::fmt::Display for Directory {
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

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Build for Directory {
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
        let dependency = builder.make_dependency(self)?;
        let root = builder.require(Root {})?;
        Ok((Some(path), dependency, vec![root]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Directory { path, .. } = self;
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
