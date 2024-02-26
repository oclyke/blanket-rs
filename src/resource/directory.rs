use std::path::{Path, PathBuf};

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
        let any = &other as &dyn std::any::Any;
        match any.downcast_ref::<Directory>() {
            Some(other) => {
                println!("comparing directories: {:?} == {:?}", self.path, other.path);
                self.path == other.path
            }
            None => false,
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
