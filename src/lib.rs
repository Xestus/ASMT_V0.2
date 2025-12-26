use std::sync::atomic::AtomicUsize;
use once_cell::sync::OnceCell;

pub mod MVCC;
pub mod btree;
pub mod storage;
pub mod transactions;
pub mod cli;
pub mod engine;
mod transaction_process_tree_fix;

// temp
pub static NODE_SIZE: OnceCell<usize> = OnceCell::new();
pub static LAST_ACTIVE_TXD: AtomicUsize = AtomicUsize::new(100);
pub static CHECKPOINT_COUNTER: AtomicUsize = AtomicUsize::new(0);
