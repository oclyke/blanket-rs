use crate::registration::Registration;
use crate::traits::Generate;

#[derive(Debug)]
pub struct Root {}

impl Generate for Root {
    fn register(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
