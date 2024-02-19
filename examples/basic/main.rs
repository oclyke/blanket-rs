// site-content structure:
// site-content/
// |-  index.html
// |-  style.css
// |-  assets/
// |    |-  * images *

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use blanket_rs::{
    builder::{Build, Builder, Dependency, Registration},
    resource::{CopyFile, Root},
};

fn main() {
    run().expect("Expected to exit successfully");
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("program start");

    let example_dir = PathBuf::from("examples/basic");
    let source = example_dir.join("site-content");
    let output = example_dir.join("site-out");
    let mut builder = Builder::new();
    builder.init()?;

    // clear the output directory
    // prefer immutability over performance
    // the directory tree will be assembled to replace the output directory
    print!("removing output directory... ");
    if output.exists() {
        std::fs::remove_dir_all(&output)?;
    }
    println!("done.");

    // register copied files
    builder.require(CopyDir::new(source.join("assets"), output.join("assets")))?;
    builder.require(CopyFile::new(
        source.join("index.html"),
        output.join("index.html"),
    ))?;
    builder.require(CopyFile::new(
        source.join("style.css"),
        output.join("style.css"),
    ))?;
    builder.require(CopyFile::new(
        source.join("reset.css"),
        output.join("reset.css"),
    ))?;

    builder.generate()?;

    println!("program end");
    Ok(())
}

#[derive(Debug)]
struct CopyDir {
    source: PathBuf,
    path: PathBuf,
}

impl CopyDir {
    fn new<P: AsRef<Path>>(source: P, path: P) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl PartialEq for CopyDir {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Build for CopyDir {
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
    ) -> Result<(Registration, Vec<Dependency>), Box<dyn std::error::Error>> {
        let path = self.path.clone();
        let dependency = builder.make_dependency(self)?;
        let root = builder.require(Root {})?;
        Ok((Registration::Concrete(dependency, path), vec![root]))
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let CopyDir { source, .. } = self;
        if !source.is_dir() {
            return Err("source is not a directory".into());
        }
        for entry in walkdir::WalkDir::new(&source) {
            let entry = entry?;
            let path = entry.path().to_path_buf();
            let relpath = path.strip_prefix(&source)?.to_path_buf();
            let output = self.path.join(relpath);
            if path.is_file() {
                let parent = match output.parent() {
                    Some(parent) => parent,
                    None => return Err("path has no parent".into()),
                };
                std::fs::create_dir_all(parent)?;
                std::fs::copy(path, output)?;
            }
        }
        Ok(())
    }
}
