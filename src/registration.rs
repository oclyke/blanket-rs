use crate::node::Node;
use crate::traits::Generate;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub enum Registration {
    RequireRoot(),
    RequireUnique(Rc<RefCell<dyn Generate>>),
    RequireShared(Rc<RefCell<Node>>),
    ReservePath(PathBuf),
}

impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Registration::RequireRoot() => write!(f, "RequireRoot"),
            Registration::RequireUnique(..) => write!(f, "RequireUnique"),
            Registration::RequireShared(..) => write!(f, "RequireShared"),
            Registration::ReservePath(path) => write!(f, "ReservePath({:?})", path),
        }
    }
}
