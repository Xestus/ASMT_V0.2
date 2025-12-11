use std::fs::File;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::cli::cli::cli;
use crate::storage::io::{empty_file, read_file};
use crate::transactions::transactions::Transaction;

pub fn initialize_from_wal(wal_file_path: &str, txd_count: Arc<RwLock<u32>>, current_transaction: Arc<RwLock<Transaction>>, file: Arc<RwLock<File>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>) {
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
                        match cli(vals.clone(), Arc::clone(&txd_count), Arc::clone(&current_transaction), Arc::clone(&file), Arc::clone(&new_node), None, Arc::clone(&all_addr)) {
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
