// site-content structure:
// site-content/
// |-  index.html
// |-  style.css
// |-  assets/
// |    |-  * images *

use std::path::PathBuf;

use blanket_rs::{
    resource::{CopyDir, CopyFile},
    Generator,
};

fn main() {
    env_logger::init();
    run().expect("Expected to exit successfully");
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("program start");

    let example_dir = PathBuf::from("examples/blanket-rs-net");
    let source = example_dir.join("site-content");
    let output = example_dir.join("site-out");
    let mut builder = Generator::new();

    // clear the output directory
    // prefer immutability over performance
    // the directory tree will be assembled to replace the output directory
    print!("removing output directory... ");
    if output.exists() {
        std::fs::remove_dir_all(&output)?;
    }
    println!("done.");

    // register copied files
    builder.require(CopyDir::builder(&source.join("assets"), &output.join("assets")).build());
    builder.require(CopyFile::new(
        source.join("index.html"),
        output.join("index.html"),
    ));
    builder.require(CopyFile::new(
        source.join("style.css"),
        output.join("style.css"),
    ));
    builder.require(CopyFile::new(
        source.join("reset.css"),
        output.join("reset.css"),
    ));

    builder.generate()?;

    println!("program end");
    Ok(())
}
