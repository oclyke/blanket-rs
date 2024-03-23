// site-content structure:
// site-content/
// |-  index.html
// |-  style.css
// |-  assets/
// |    |-  * images *

use std::path::PathBuf;

use blanket_rs::{
    generator::{CopyDir, CopyFile},
    Generator,
};

fn main() {
    run().expect("Expected to exit successfully");
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("program start");

    let example_dir = PathBuf::from("examples/blanket-rs-net");
    let source = example_dir.join("site-content");
    let output = example_dir.join("site-out");

    // create the generator
    // use the allow filter to protect from writing above the output directory
    let mut site = Generator::builder()
        .allow(vec![format!("{}/.*", output.to_string_lossy()).as_str()])
        .build();

    // clear the output directory
    // prefer immutability over performance
    // the directory tree will be assembled to replace the output directory
    print!("removing output directory... ");
    if output.exists() {
        std::fs::remove_dir_all(&output)?;
    }
    println!("done.");

    // register copied files
    site.require(CopyDir::builder(&source.join("assets"), &output.join("assets")).build())?;
    site.require(CopyFile::new(
        &source.join("index.html"),
        &output.join("index.html"),
    ))?;
    site.require(CopyFile::new(
        &source.join("style.css"),
        &output.join("style.css"),
    ))?;
    site.require(CopyFile::new(
        &source.join("reset.css"),
        &output.join("reset.css"),
    ))?;

    println!("generating tagets:");
    for target in site.targets() {
        println!("  {:?}", target);
    }
    site.generate()?;

    println!("program end");
    Ok(())
}
