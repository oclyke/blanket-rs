use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{html::Render, Generate};

pub struct RenderFile {
    element: Rc<RefCell<dyn Render>>,
}

impl RenderFile {
    pub fn new(element: Rc<RefCell<dyn Render>>) -> Self {
        Self {
            element,
        }
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
