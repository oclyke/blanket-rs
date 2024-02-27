use crate::{Generate, Registration};

impl Generate for String {
    fn register(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
