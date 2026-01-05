use std::collections::HashMap;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::cli::cli::cli;
use crate::storage::io::{empty_file, read_file};
use crate::transactions::transactions::Transaction;

pub fn initialize_from_wal(wal_file_path: &str, txd_count: Arc<RwLock<u32>>, vid_count: Arc<RwLock<u32>>,prev_lsn: Arc<RwLock<u64>>, current_transaction: Arc<RwLock<Transaction>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, ts_ord: Arc<RwLock<HashMap<u32, u32>>>) {
    match read_file(wal_file_path) {
        Ok(value) => {
            let mut uncommitted_strings = Vec::new();
            let mut load_to_cli = false;
            for items in value.lines() {
                let items = items.replace("\"", "");

                uncommitted_strings.push(items.clone());
                if items.to_lowercase().contains("commit") {
                    load_to_cli = true;
                }

                if load_to_cli {
                    for vals in uncommitted_strings.iter() {
                        match cli(vals.clone(), Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn),Arc::clone(&current_transaction), Arc::clone(&new_node), None, Arc::clone(&all_addr), Arc::clone(&ts_ord)) {
                            Ok(_) => {}
                            Err(e) => println!("WAL recovery error: {}", e),

                        }
                    }
                    load_to_cli = false;

                    uncommitted_strings.clear();
                }
            }

            match empty_file(wal_file_path) {
                Ok(_) => {}
                Err(e) => println!("File truncation error: {}", e),
            }
        }
        Err(e) => println!("{}", e),
    }
}

pub fn real_initialize_from_wal() {

}
