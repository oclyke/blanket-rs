use crate::{
    registration::{Registration, TerminalRegistration}, DelayedRegistration, Generate, ResourceRef
};

impl Generate for String {
    fn register(&self, resource: ResourceRef) -> DelayedRegistration {
        Box::new(move || {
            Ok(vec![Registration::Terminal(TerminalRegistration::Virtual(resource))])
        })
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
