use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{html::Render, Generate, Register, Registration};

pub struct RenderFile {
    element: Rc<RefCell<dyn Render>>,
    destination: PathBuf,
}

impl RenderFile {
    pub fn new(destination: &PathBuf, element: Rc<RefCell<dyn Render>>) -> Self {
        Self {
            element,
            destination: destination.clone(),
        }
    }
}

impl Register for RenderFile {
    fn register(self) -> Result<Registration, Box<dyn std::error::Error>> {
        Ok(Registration::Terminal {
            path: self.destination.clone(),
            generator: Rc::new(RefCell::new(self)),
        })
    }
}

impl Generate for RenderFile {
    fn generate(&self, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let dir = output.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::write(output, self.element.borrow().render()?)?;
        Ok(())
    }
}
