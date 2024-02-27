use std::path::{Path, PathBuf};
use std::any::Any;

use crate::{
    DelayedRegistration, Generate, ObjectRef, Registration, ResourceRef, TerminalRegistration,
};

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

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Generate for Directory {
    fn equals(&self, other: ObjectRef) -> bool {
        let borrowed = other.borrow();
        let any_ref = &*borrowed as &dyn Any;
        if let Some(specific) = any_ref.downcast_ref::<Directory>() {
            self.path == specific.path
        } else {
            false
        }
    }
    fn register(&self, resource: ResourceRef) -> DelayedRegistration {
        let path = self.path.clone();
        Box::new(move || {
            Ok(vec![Registration::Terminal(
                TerminalRegistration::Concrete(resource, path),
            )])
        })
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Directory { path, .. } = self;
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
