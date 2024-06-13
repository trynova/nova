#![cfg_attr(feature = "no_std", no_std, feature(error_in_core))]

#[cfg(feature = "no_std")]
use core2::io;
#[cfg(feature = "no_std")]
extern crate alloc;
#[cfg(not(feature = "no_std"))]
use std::io;

mod decoder;
pub mod error;
mod varint;

pub fn compile_module<R: io::Read>(bytes: &mut R) -> Result<(), error::Error> {
    let _module = decoder::Module::new(bytes)?;
    todo!("Still need to add compiler and export generator");
}
