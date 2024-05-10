// site-content structure:
// site-content/
// |-  index.html
// |-  style.css
// |-  assets/
// |    |-  * images *

use std::path::PathBuf;

use blanket_rs::{
    targets::{CopyDir, CopyFile},
    Generator,
};

fn main() {
    run().expect("Expected to exit successfully");
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("program start");

    let example_dir = PathBuf::from("examples/basic");
    let source = example_dir.join("site-content");
    let output = example_dir.join("site-out");
    let mut site = Generator::new();

    // list of desired targets
    site.add_target(CopyFile::new(
        source.join("index.html"),
        output.join("index.html"),
    ));
    site.add_target(CopyFile::new(
        source.join("style.css"),
        output.join("style.css"),
    ));
    site.add_target(CopyFile::new(
        source.join("reset.css"),
        output.join("reset.css"),
    ));
    site.add_targets(
        CopyDir::builder(&source.join("assets"), &output.join("assets"))
            .include(vec![r".*/.*\.png"])
            .build()
            .targets(),
    );

    // generate the site without caching
    site.generate(blanket_rs::cache::NoCache {})?;

    println!("program end");
    Ok(())
}
