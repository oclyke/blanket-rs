use crate::builder::{Build, Builder, Dependency};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct Root {}

impl std::fmt::Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "--root--")
    }
}

impl Build for Root {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn equals(&self, other: Rc<RefCell<dyn Build>>) -> bool {
        let other = other.borrow();
        let any = other.as_any();
        match any.downcast_ref::<Self>() {
            Some(_) => true,
            None => false,
        }
    }
    fn register(
        self,
        builder: &mut Builder,
    ) -> Result<(Option<PathBuf>, Dependency, Vec<Dependency>), Box<dyn std::error::Error>> {
        let dependency = builder.make_dependency(self)?;
        Ok((Some(PathBuf::from("/")), dependency, vec![]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Root::generate");
        Ok(())
    }
}
