mod context;
mod device;

pub use context::*;
pub use device::*;

use std::error;
use std::fmt;
use std::result;

#[derive(Debug)]
pub enum Error {
    Creation,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {}

type Result<T> = result::Result<T, Error>;
