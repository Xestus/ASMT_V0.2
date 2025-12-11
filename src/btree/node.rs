use std::sync::{Arc, RwLock};
use crate::MVCC::versions::Version;

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Items {
    pub key: u32,
    pub rank: u32,
    pub version: Vec<Version>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub input: Vec<Items>,
    pub rank: u32,
    pub children: Vec<Arc<RwLock<Node>>>,
}

impl Node {
    /// The function creates a new empty node for a B-tree during initialization and operations such as splitting.
    /// The default value of field `rank` is 1 as:
    /// - A new B-Tree always starts with an empty Node with `rank: 1`.
    /// - Any Node that isn't root node has it's rank default to `parent's rank + 1`.
    ///
    /// The fields `input` and `children` are initialized as empty Vector as:
    /// - A new B-Tree's root node will always have 0 items and 0 children.
    /// - Any new Node from splitting would have its input and children derived from its predecessor Node.
    ///
    /// The new Node is wrapped with Arc and Mutex as Mutex allows Thread-safe mutations and Arc allows Mutex to be shared across threads.
    /// As the B-Tree will eventually scale up to concurrency, Arc<Mutex<T>> helps in future proofing the concept.
    ///
    pub fn new() -> Arc<RwLock<Node>> {
        let instance = Arc::new(RwLock::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
        }));
        instance
    }
}