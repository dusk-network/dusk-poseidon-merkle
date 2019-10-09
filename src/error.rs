use std::{error, fmt};

#[derive(Debug, Copy, Clone)]
/// Possible error states for the hashing.
pub enum Error {
    /// The allowed number of leaves cannot be greater than the arity of the tree.
    FullBuffer,
    /// Attempt to reference an index element that is out of bounds
    IndexOutOfBounds,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::FullBuffer => write!(
                f,
                "The size of the buffer cannot be greater than the arity of the merkle tree."
            ),
            Error::IndexOutOfBounds => write!(f, "The referenced index is outs of bounds."),
        }
    }
}
