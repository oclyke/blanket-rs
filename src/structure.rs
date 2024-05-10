use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub enum Node {
    File,
    Directory,
}

pub type Structure = HashMap<PathBuf, Node>;

pub fn initialize(structure: &mut Structure) {
    structure.insert(PathBuf::from(""), Node::Directory);
}

pub fn add_node(structure: &mut Structure, path: &Path, node: Node) {
    match structure.get(path) {
        Some(Node::File) => {
            panic!("Cannot add a node to a file");
        }
        Some(Node::Directory) => match node {
            Node::File => {
                panic!("An existing directory cannot be replaced with a file")
            }
            Node::Directory => {
                // do nothing
            }
        },
        None => {
            // add the parent as a directory
            let parent = match path.parent() {
                Some(parent) => parent,
                None => {
                    panic!("Given path has no parent");
                }
            };
            add_node(structure, parent, Node::Directory);

            // add the node
            structure.insert(path.into(), node);
        }
    }
}
