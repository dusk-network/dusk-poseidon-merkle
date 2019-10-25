use crate::Error;

use std::convert::{TryFrom, TryInto};

use rocksdb::DB;
use serde::{Deserialize, Serialize};

/// Representation of a coordinate inside the tree.
///
/// No tree consistency is performed in this layer. This implies invalid coordinates are possible
/// inside a tree.
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct MerkleCoord {
    /// Height position in the tree
    pub height: usize,
    /// Index for the current row of the tree.
    pub idx: usize,
}

impl MerkleCoord {
    /// MerkleCoord constructor
    pub fn new(height: usize, idx: usize) -> Self {
        MerkleCoord { height, idx }
    }

    /// Attempt to fetch a leaf from a DB
    pub fn fetch_leaf<T>(self, db: &DB) -> Result<Option<T>, Error>
    where
        T: for<'a> Deserialize<'a>,
    {
        let coord: Vec<u8> = self.try_into()?;

        db.get(coord.as_slice())
            .map_err(|e| Error::Other(e.to_string()))?
            .map(|b| bincode::deserialize::<T>(b.as_ref()).map_err(|e| Error::Other(e.to_string())))
            .transpose()
    }

    /// Attempt to persist a leaf into a DB
    pub fn persist_leaf<T>(self, db: &DB, leaf: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let coord: Vec<u8> = self.try_into()?;
        let leaf = bincode::serialize(&leaf).map_err(|e| Error::Other(e.to_string()))?;

        db.put(coord.as_slice(), leaf.as_slice())
            .map_err(|e| Error::Other(e.to_string()))
    }
}

impl TryFrom<&[u8]> for MerkleCoord {
    type Error = Error;

    fn try_from(buf: &[u8]) -> Result<MerkleCoord, Self::Error> {
        bincode::deserialize(&buf).map_err(|e| Error::Other(e.to_string()))
    }
}

impl TryInto<Vec<u8>> for MerkleCoord {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        bincode::serialize(&self).map_err(|e| Error::Other(e.to_string()))
    }
}
