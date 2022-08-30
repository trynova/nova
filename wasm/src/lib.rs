mod decoder;
pub mod error;
mod varint;

pub fn compile_module<R: std::io::Read>(bytes: &mut R) -> Result<(), error::Error> {
    let _module = decoder::Module::new(bytes)?;
    todo!("Still need to add compiler and export generator");
}
