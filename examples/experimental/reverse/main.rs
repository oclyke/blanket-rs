#![feature(trait_upcasting)]

use blanket_rs::{
    Generate,
    Generator,
    Registration,
    resource::Directory,
};

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn main() {
    run().expect("Expected to exit successfully");
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("program start");

    let example_dir = PathBuf::from("examples/experimental/reverse");
    let output = example_dir.join("site-out");
    let mut generator = Generator::new();

    // clear the output directory
    // prefer immutability over performance
    // the directory tree will be assembled to replace the output directory
    print!("removing output directory... ");
    if output.exists() {
        std::fs::remove_dir_all(&output)?;
    }
    println!("done.");

    // this special dir works inside out.
    // the dir wraps the files, but the files depend on the dir.
    // this is a step along the path to making functional style resources.
    generator.require(Dir{
        path: output.join("reverse_dir"),
        entries: vec![
            Rc::new(RefCell::new(Fil{path: PathBuf::from("zany_file1.txt")})),
            Rc::new(RefCell::new(Fil{path: PathBuf::from("zany_file2.txt")})),
            Rc::new(RefCell::new(Fil{path: PathBuf::from("oops, this file has a bad name.txt")})),
        ],
    })?;

    // to illustrate, let's make a functional version of the same demo
    let mut fDir = |path: PathBuf, entries: Vec<Rc<RefCell<dyn DirEntry>>> | {
        generator.require(Dir{path, entries})
    };

    fDir(
        output.join("functional_dir"),
        vec![
            Rc::new(RefCell::new(Fil{path: PathBuf::from("it might not seem much different.txt")})),
            Rc::new(RefCell::new(Fil{path: PathBuf::from("but because the generator is captured in the closure.cont")})),
            Rc::new(RefCell::new(Fil{path: PathBuf::from("it is possible to chain the resources together")})),
            Rc::new(RefCell::new(Fil{path: PathBuf::from("I wonder what it would take to get the syntactic sugar like React or Render...")})),
        ]
    )?;

    generator.generate()?;

    println!("program end");
    Ok(())
}

trait DirEntry: Generate {
    fn prefix_path(&mut self, prefix: PathBuf);
}

#[derive(Debug)]
struct Dir {
    entries: Vec<Rc<RefCell<dyn DirEntry>>>,
    path: PathBuf,
}

impl Generate for Dir {
    fn register(&mut self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        let directory = Rc::new(RefCell::new(Directory::new(self.path.clone())));
        let mut registrations = vec![
            Registration::RequireUnique(directory)
        ];
        for entry in &self.entries {
            // Note: this feels like an ugly imperative way of doing this which does not
            //       keep the Fil and Dir structs isolated. One day the idea would be to
            //       use a concept of "context providers" to enable the Dir to influence
            //       the Fil's generation.
            entry.borrow_mut().prefix_path(self.path.clone());

            // Note: testing out the "PrecedeX" registration. Supposed to indicate that
            //       this node (the Dir) should be generated before the DirEntry nodes.
            //       Working in concert with the "RequireUnique" registration on the
            //       Directory node, this should ensure that the directory is created
            //       before the files are written.
            registrations.push(Registration::PrecedeUnique(entry.clone()));
        }
        Ok(registrations)
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("creating directory: {:?}", self.path);
        Ok(())
    }
}

#[derive(Debug)]
struct Fil {
    path: PathBuf,
}

impl Generate for Fil {
    fn register(&mut self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("writing file: {:?}", self.path);
        std::fs::write(&self.path, "Hello, world!")?;
        Ok(())
    }
}

impl DirEntry for Fil {
    fn prefix_path(&mut self, prefix: PathBuf) {
        self.path = prefix.join(&self.path);
    }
}
