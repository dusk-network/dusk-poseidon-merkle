use crate::{Error, Poseidon, PoseidonLeaf, Scalar, MERKLE_ARITY};

use std::cmp;
use std::convert::TryInto;
use std::ops;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use rocksdb::DB;
#[cfg(test)]
use tempdir::TempDir;

pub use merkle_coord::MerkleCoord;
pub use merkle_range::MerkleRange;
pub use proof::BigProof;

const CACHE_HEIGHT_INTERVAL: usize = 2;

mod merkle_coord;
mod merkle_range;
mod proof;

/// The merkle tree will accept up to `MERKLE_ARITY * MERKLE_WIDTH` leaves.
#[derive(Debug)]
pub struct BigMerkleTree {
    width: usize,
    height: usize,
    max_idx: usize,
    /// For most cases, this attribute should hold one element that represents the higher idx to
    /// the end of the tree. The usage of the free intervals is, however, non-restricted.
    empty_intervals: Vec<MerkleRange>,
    db: Arc<DB>,
    cache: Arc<DB>,
}

impl Clone for BigMerkleTree {
    fn clone(&self) -> Self {
        BigMerkleTree {
            max_idx: self.max_idx,
            db: Arc::clone(&self.db),
            cache: Arc::clone(&self.cache),
            empty_intervals: self.empty_intervals.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

impl BigMerkleTree {
    /// `BigMerkleTree` constructor
    pub fn new<D: AsRef<Path>, E: AsRef<Path>>(
        db_path: D,
        cache_path: E,
        width: usize,
    ) -> Result<Self, Error> {
        let max_idx = 0;
        let height = width as f64;
        let height = height.log(MERKLE_ARITY as f64) as usize;

        let mut empty_intervals = Vec::new();

        let db = DB::open_default(db_path).map_err(|e| Error::Other(e.to_string()))?;
        let db = Arc::new(db);

        let cache = DB::open_default(cache_path).map_err(|e| Error::Other(e.to_string()))?;
        let cache = Arc::new(cache);

        // The initial empty interval is the whole input set. Therefore, the relative range for the
        // root node.
        empty_intervals.push(MerkleRange::new(height, 0, 0));

        Ok(BigMerkleTree {
            max_idx,
            db,
            cache,
            empty_intervals,
            width,
            height,
        })
    }

    /// Return a reference to the internal path of the DB
    pub fn db_path(&self) -> &Path {
        self.db.path()
    }

    /// Height of the tree
    pub fn height(&self) -> usize {
        self.height
    }

    /// Arity of the tree
    pub fn arity(&self) -> usize {
        MERKLE_ARITY
    }

    /// Width of the tree
    pub fn width(&self) -> usize {
        self.width
    }

    /// Divide the tree into a parallelizable path to the root
    pub fn segments(&self) -> Vec<MerkleCoord> {
        let mut coords = vec![];
        let mut coord = MerkleCoord::new(self.height, self.max_idx);

        while coord.height > 0 {
            coord.descend(CACHE_HEIGHT_INTERVAL);

            for i in 0..coord.idx + 1 {
                coords.push(MerkleCoord::new(coord.height, i));
            }
        }

        coords
    }

    /// Check if the node in the provided height and index belongs to an empty super tree.
    pub fn node_is_empty(&self, height: usize, idx: usize) -> bool {
        let r = MerkleRange::new(self.height, height, idx);
        self.empty_intervals.contains(&r)
    }

    /// Insert the provided leaf on the provided index
    pub fn insert<T: PoseidonLeaf>(&mut self, idx: usize, leaf: T) -> Result<(), Error> {
        self.insert_height(self.height, idx, leaf)
    }

    /// Insert the provided leaf on the provided index
    fn insert_height<T: PoseidonLeaf>(
        &mut self,
        height: usize,
        idx: usize,
        leaf: T,
    ) -> Result<(), Error> {
        let coord = MerkleCoord::new(height, idx);

        if height == self.height {
            coord
                .persist_leaf(&self.db, leaf)
                .and_then(|_| self.inserted(idx))
        } else {
            coord.persist_leaf(&self.cache, leaf)
        }
    }

    /// Flag the provided index as inserted in the structure.
    ///
    /// This will reorganize the empty intervals.
    pub fn inserted(&mut self, idx: usize) -> Result<(), Error> {
        self.max_idx = cmp::max(self.max_idx, idx);

        // Should split the empty interval only if the current idx belongs to an empty base
        if self.node_is_empty(self.height, idx) {
            // The range for the current idx is always itself + 1, since its possible to insert
            // leaves only on the base
            let idx_r: MerkleRange = (idx..idx + 1).into();

            let mut r1 = None;
            let mut r2 = None;
            let mut empty_idx = None;

            // Find the empty interval that should be split
            for (i, r) in self.empty_intervals.iter().enumerate() {
                if r == &idx_r {
                    r1 = Some(r.clone());
                    r2 = Some(r.clone());
                    empty_idx = Some(i);
                }
            }

            // If the interval is not found, then we have unreachable code since the
            // `node_is_empty` check was performed
            let mut r1 = r1.ok_or(Error::IndexOutOfBounds)?;
            let mut r2 = r2.ok_or(Error::IndexOutOfBounds)?;
            let empty_idx = empty_idx.ok_or(Error::IndexOutOfBounds)?;

            // The rightmost of the interval is always split
            r2.0.start = idx + 1;
            self.empty_intervals[empty_idx] = r2;

            // The leftmost of the interval is split only if idx is not the first element of the
            // provided interval
            //
            // Since the base should be, but not necessarily is, append only, this should lead to
            // performance degradation
            if idx > r1.0.start {
                r1.0.end = idx;
                self.empty_intervals.push(r1);
            }
        }

        self.modified(idx)
    }

    /// Set the provided leaf index as absent for the hash calculation.
    pub fn remove(&mut self, idx: usize) -> Result<(), Error> {
        let coord: Vec<u8> = MerkleCoord::new(self.height, idx).try_into()?;

        self.db
            .delete(coord.as_slice())
            .map_err(|e| Error::Other(e.to_string()))
            .and_then(|_| self.removed(idx))
    }

    /// Flag the provided index as absent.
    ///
    /// This will reorganize the empty intervals.
    pub fn removed(&mut self, idx: usize) -> Result<(), Error> {
        // Check if there is an adjacent left interval
        let left = idx > 0 && self.node_is_empty(self.height, idx - 1);

        // Check if there is an adjacent right interval
        let right = idx < self.width - 1 && self.node_is_empty(self.height, idx + 1);

        if left && right {
            // Merge the two intervals

            // Fetch the index of the left and right intervals
            let r1 = MerkleRange::from(idx - 1..idx);
            let r2 = MerkleRange::from(idx + 1..idx + 2);
            let mut idx_left = None;
            let mut idx_right = None;
            for (i, r) in self.empty_intervals.iter().enumerate() {
                if r == &r1 {
                    idx_left = Some(i);
                }

                if r == &r2 {
                    idx_right = Some(i);
                }
            }

            // If the interval is not found, then we have unreachable code since the
            // `node_is_empty` check was performed
            let idx_left = idx_left.ok_or(Error::IndexOutOfBounds)?;
            let idx_right = idx_right.ok_or(Error::IndexOutOfBounds)?;

            self.empty_intervals[idx_left].0.end = self.empty_intervals[idx_right].0.end;
            self.empty_intervals.remove(idx_right);
        } else if left {
            // Decrement the left interval
            let r1 = MerkleRange::from(idx - 1..idx);
            let mut r1_idx = None;

            for (i, r) in self.empty_intervals.iter().enumerate() {
                if r == &r1 {
                    r1_idx = Some(i);
                }
            }

            // If the interval is not found, then we have unreachable code since the
            // `node_is_empty` check was performed
            let r1_idx = r1_idx.ok_or(Error::IndexOutOfBounds)?;

            // Decrement the range, if the number of elements is greater than 1
            if self.empty_intervals[r1_idx].0.end == self.empty_intervals[r1_idx].0.start + 1 {
                self.empty_intervals.remove(r1_idx);
            } else {
                self.empty_intervals[r1_idx].0.end -= 1;
            }
        } else if right {
            // Increment the left start interval
            let r1 = MerkleRange::from(idx + 1..idx + 2);
            let mut r1_idx = None;

            for (i, r) in self.empty_intervals.iter().enumerate() {
                if r == &r1 {
                    r1_idx = Some(i);
                }
            }

            // If the interval is not found, then we have unreachable code since the
            // `node_is_empty` check was performed
            let r1_idx = r1_idx.ok_or(Error::IndexOutOfBounds)?;
            self.empty_intervals[r1_idx].0.start += 1;

            // Increments the range, if the number of elements is greater than 1
            if self.empty_intervals[r1_idx].0.end == self.empty_intervals[r1_idx].0.start + 1 {
                self.empty_intervals.remove(r1_idx);
            } else {
                self.empty_intervals[r1_idx].0.start += 1;
            }
        } else {
            // If there is no adjacent empty interval, then create an interval of its own
            self.empty_intervals.push((idx..idx + 1).into());
        }

        self.modified(idx)
    }

    /// Flag the base idx as modified, and delete all sub-trees from the cache
    fn modified(&mut self, idx: usize) -> Result<(), Error> {
        let mut coord = MerkleCoord::new(self.height, idx);

        loop {
            coord.descend(1);

            let c: Vec<u8> = coord.try_into()?;
            self.cache
                .delete(c.as_slice())
                .map_err(|e| Error::Other(e.to_string()))?;

            if coord.height == 0 {
                break;
            }
        }

        Ok(())
    }

    /// Fetch a node of the tree for the provided coordinates
    pub fn node<T: PoseidonLeaf>(&mut self, height: usize, idx: usize) -> Result<Option<T>, Error>
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        if height == self.height {
            // Fetch directly from db
            MerkleCoord::new(height, idx).fetch_leaf(&self.db)
        } else if self.node_is_empty(height, idx) {
            // Fetch a precalculated null node
            if height == self.height {
                Ok(None)
            } else {
                // TODO Generate a precalculated height for null sub-trees
                Ok(Some(T::from(0u64)))
            }
        } else {
            // Calculate the node
            let coord = MerkleCoord::new(height, idx);
            let should_cache = (height % CACHE_HEIGHT_INTERVAL) == 0;

            let node = if should_cache {
                coord.fetch_leaf::<T>(&self.cache)?
            } else {
                None
            };

            if node.is_some() {
                return Ok(node);
            }

            let mut h = Poseidon::default();

            let needle = idx * MERKLE_ARITY;
            for i in 0..MERKLE_ARITY {
                if let Some(n) = self.node(height + 1, needle + i)? {
                    h.insert_unchecked(i, n);
                }
            }

            let node = h.hash();
            if should_cache {
                coord.persist_leaf(&self.cache, node)?;
            }

            Ok(Some(node))
        }
    }

    /// Generate a proof of membership for the provided leaf index
    pub fn proof<T: PoseidonLeaf>(&mut self, mut needle: usize) -> Result<BigProof<T>, Error>
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        let mut proof = BigProof::new();
        let mut leaves = [None; MERKLE_ARITY];

        for row in 0..self.height {
            let from = MERKLE_ARITY * (needle / MERKLE_ARITY);
            let idx = needle % MERKLE_ARITY;

            for i in 0..MERKLE_ARITY {
                leaves[i] = self.node(self.height - row, from + i)?;
            }

            proof.push(idx, leaves);
            needle /= MERKLE_ARITY;
        }

        Ok(proof)
    }

    /// Calculate and return the root of the merkle tree.
    pub fn root<T: PoseidonLeaf>(&mut self) -> Result<T, Error>
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        let (tx, rx) = mpsc::channel();
        let rx = Mutex::new(rx);
        let rx = Arc::new(rx);

        let mut handles: Vec<thread::JoinHandle<Result<(), Error>>> = vec![];

        let segments = self.segments();
        for s in segments {
            tx.send(s).map_err(|e| Error::Other(e.to_string()))?;
        }

        for _ in 0..num_cpus::get() {
            let worker = Arc::clone(&rx);
            let mut tree = self.clone();

            handles.push(thread::spawn(move || {
                while let Some(c) = worker
                    .lock()
                    .map(|r| r.recv().ok())
                    .map_err(|e| Error::Other(e.to_string()))?
                {
                    tree.node(c.height, c.idx)?;
                }

                Ok(())
            }));
        }

        for h in handles {
            h.join().unwrap().unwrap();
        }

        self.node(0, 0).and_then(|n| {
            n.ok_or(Error::Other(
                "It was not possible to obtain the root node from the merkle tree.".to_owned(),
            ))
        })
    }
}

#[cfg(test)]
pub fn big_merkle_default(path: &str) -> BigMerkleTree {
    // 2^34
    let width = 17179869184;
    let db_path = TempDir::new(path).map(|t| t.into_path()).unwrap();

    let cache_path = format!("{}-cache", path);
    let cache_path = TempDir::new(cache_path.as_str())
        .map(|t| t.into_path())
        .unwrap();

    BigMerkleTree::new(db_path, cache_path, width).unwrap()
}

#[cfg(test)]
mod tests {
    use super::big_merkle_default;

    #[test]
    fn big_merkle_empty() {
        let mut merkle = big_merkle_default("big_merkle_empty");
        let idx = merkle.width() / 3;

        assert!(merkle.node_is_empty(0, 0));
        assert!(merkle.node_is_empty(merkle.height(), idx));

        merkle.inserted(idx).unwrap();

        assert!(!merkle.node_is_empty(0, 0));
        assert!(!merkle.node_is_empty(merkle.height(), idx));
        assert!(merkle.node_is_empty(merkle.height(), idx - 1));
        assert!(merkle.node_is_empty(merkle.height(), idx + 1));

        merkle.inserted(0).unwrap();
        assert!(!merkle.node_is_empty(merkle.height(), 0));
    }
}
