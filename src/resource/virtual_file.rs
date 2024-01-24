use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::builder::{Build, Builder, Dependency};

pub struct VirtualFile {
    path: PathBuf,
    content: Option<String>,
}

impl VirtualFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            content: None,
        }
    }
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }
}

impl std::fmt::Debug for VirtualFile {
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

impl PartialEq for VirtualFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Build for VirtualFile {
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
        Ok((Some(path), dependency, vec![]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let VirtualFile { path, .. } = self;
        self.content = Some(std::fs::read_to_string(path)?);
        Ok(())
    }
}

pub fn extract_content(dependency: &Dependency) -> Result<String, Box<dyn std::error::Error>> {
    let resource = dependency.resource();
    let resource = resource.borrow();
    let any = resource.as_any();
    let resource = match any.downcast_ref::<VirtualFile>() {
        Some(resource) => resource,
        None => return Err("resource is not a virtual file".into()),
    };
    let content = match resource.content() {
        Some(content) => content.to_string(),
        None => return Err("resource has no content".into()),
    };
    Ok(content)
}
