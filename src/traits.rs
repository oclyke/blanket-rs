use crate::registration::Registration;

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// An object that can be built.
pub trait Generate: Any + std::fmt::Debug {
    /// Registers the object with the builder.
    /// Uses a delayed registration which is evaluated lazily at build time.
    fn register(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>>;

    /// Generate the object.
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns true if the object is equal to the other object.
    /// Used to allow output location sharing for compatible objects.
    /// Generally this should return false unless `other` can be downcast to
    /// `Self`.
    fn equals(&self, _other: Rc<RefCell<dyn Generate>>) -> bool {
        false
    }
}
