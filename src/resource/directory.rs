use crate::{Generate, Registration};

use std::any::Any;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
    fn equals(&self, other: Rc<RefCell<dyn Generate>>) -> bool {
        let borrowed = other.borrow();
        let any_ref = &*borrowed as &dyn Any;
        if let Some(specific) = any_ref.downcast_ref::<Directory>() {
            self.path == specific.path
        } else {
            false
        }
    }
    fn register(&mut self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        Ok(vec![Registration::ReservePath(self.path.clone())])
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Directory { path, .. } = self;
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
