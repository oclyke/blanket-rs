use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::generator::{Generate, Generator, Node, Registration};

#[derive(Debug)]
pub struct Directory {
    id: Option<u64>,
    path: PathBuf,
}

impl Directory {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            id: None,
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn equals(&self, other: Rc<RefCell<dyn Generate>>) -> bool {
        let other = other.borrow();
        let any = other.as_any();
        match any.downcast_ref::<Self>() {
            Some(other) => {
                self == other
            }
            None => false,
        }
    }
    fn id(&self) -> Option<u64> {
        None
    }
    fn register(&mut self, id: u64) -> Result<Registration, Box<dyn std::error::Error>> {
        self.id = Some(id);
        Ok(Registration::Concrete(self.path.clone()))
    }
    fn dependencies(
        &mut self,
        _builder: &mut Generator,
    ) -> Result<Vec<Node>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Directory { path, .. } = self;
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
