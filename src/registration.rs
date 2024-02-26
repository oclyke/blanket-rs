use crate::{ResourceRef, ObjectRef};

use std::path::PathBuf;

pub type DelayedRegistration =
    Box<dyn FnOnce() -> Result<Vec<Registration>, Box<dyn std::error::Error>>>;

pub enum NonterminalRegistration {
    Delayed(DelayedRegistration),
    DependUnique(ResourceRef, ObjectRef),
    DependShared(ResourceRef, ResourceRef),
}

impl std::fmt::Debug for NonterminalRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NonterminalRegistration::Delayed(_) => {
                write!(f, "Delayed")
            }
            NonterminalRegistration::DependUnique(..) => {
                write!(f, "DependUnique")
            }
            NonterminalRegistration::DependShared(..) => {
                write!(f, "DependShared")
            }
        }
    }
}

pub enum TerminalRegistration {
    Concrete(ResourceRef, PathBuf),
    Virtual(ResourceRef),
}

impl std::fmt::Debug for TerminalRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalRegistration::Concrete(_, _) => {
                write!(f, "Concrete")
            }
            TerminalRegistration::Virtual(_) => {
                write!(f, "Virtual")
            }
        }
    }
}

pub enum Registration {
    Nonterminal(NonterminalRegistration),
    Terminal(TerminalRegistration),
}

impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Registration::Nonterminal(nonterminal) => {
                write!(f, "Nonterminal::{:?}", nonterminal)
            }
            Registration::Terminal(terminal) => {
                write!(f, "Terminal::{:?}", terminal)
            }
        }
    }
}
