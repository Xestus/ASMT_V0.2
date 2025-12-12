use std::fs::File;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use crate::btree::node::Node;
use crate::CHECKPOINT_COUNTER;
use crate::MVCC::gc::remove_dead_version;
use crate::MVCC::snapshot::snapshot;
use crate::storage::io::empty_file;
use crate::storage::ser::serialize;
use crate::storage::wal::reader::get_uncommitted_transactions;
use crate::storage::wal::writer::flush_to_wal;
use crate::transactions::manager::get_all_active_transaction;
use crate::transactions::transactions::Transaction;

pub fn checkpoint(node: Arc<RwLock<Node>>, serialized_file_path: &str, wal_file_path: &str, file: Arc<RwLock<File>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, transaction: Arc<RwLock<Transaction>> ) {
    let all_active_txd =  get_all_active_transaction(transaction, all_addr);
    
    if all_active_txd.len() == 0 { 
        println!("The given transaction has no active transaction. That's odd. HMM");
    } else {
        let x = all_active_txd.first().unwrap();
        remove_dead_version(Arc::clone(&node), *x);
    }
    
    let mut cloned_node = node.clone();

    cloned_node = snapshot(cloned_node, None);

    match serialize(Arc::clone(&cloned_node),serialized_file_path) {
        Ok(_) => {}
        Err(e) =>  println!("Serialization failed: {}", e),
    }

    match get_uncommitted_transactions(wal_file_path) {
        Ok(uncommitted_strings) => match empty_file(wal_file_path) {
            Ok(_) => {
                for strs in uncommitted_strings.iter() {
                    let uncommitted_args = strs.split(" ").collect::<Vec<&str>>();
                    match flush_to_wal(Arc::clone(&file), uncommitted_args) {
                        Ok(_) => {}
                        Err(e) => println!("Flushing to WAL failed: {}", e),
                    }
                }
            }
            Err(e) => println!("WAL truncation error: {}", e),
        },
        Err(e) => println!("Can't fetch uncommitted transactions: {}", e),
    }

    println!("{:?}", cloned_node.read().unwrap().print_tree());

    CHECKPOINT_COUNTER.store(0, Ordering::Relaxed);
}
