use crate::Generate;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub trait Render {
    fn render(&self) -> Result<String, Box<dyn std::error::Error>>;
}

pub struct ElementFragment {
    pub children: Vec<Rc<RefCell<dyn Render>>>,
}
impl Render for ElementFragment {
    fn render(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.children
            .iter()
            .try_fold(String::new(), |mut acc, child| {
                let rendered = child.borrow().render()?;
                acc.push_str(&rendered);
                Ok(acc)
            })
    }
}

impl Generate for ElementFragment {
    fn generate(&self, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = self.render()?;
        let dir = output.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::write(output, content)?;
        Ok(())
    }
}

impl From<SimpleElement> for ElementFragment {
    fn from(element: SimpleElement) -> Self {
        ElementFragment {
            children: vec![Rc::new(RefCell::new(element))],
        }
    }
}

pub struct SimpleElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub content: Option<String>,
    pub children: Vec<Rc<RefCell<dyn Render>>>,
}
impl Render for SimpleElement {
    fn render(&self) -> Result<String, Box<dyn std::error::Error>> {
        let attributes = self
            .attributes
            .iter()
            .map(|(key, value)| format!(" {}=\"{}\"", key, value))
            .collect::<Vec<String>>()
            .join(" ");

        let content = match self.content {
            Some(ref content) => content.clone(),
            None => ElementFragment {
                children: self.children.clone(),
            }
            .render()?,
        };
        Ok(format!(
            "<{}{}>{}</{}>",
            self.tag, attributes, content, self.tag
        ))
    }
}

pub struct HTML5Doctype;
impl Render for HTML5Doctype {
    fn render(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("<!DOCTYPE html>".into())
    }
}

pub struct DangerouslySetInnerHTML {
    pub html: String,
}
impl Render for DangerouslySetInnerHTML {
    fn render(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.html.clone())
    }
}
