use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::cli::cli::cli;
use crate::storage::io::empty_file;
use crate::storage::wal::log_utils::{create_command};
use crate::storage::wal::record::RecordStruct;
use crate::transactions::transactions::Transaction;

// WAL entry isn't logged back in as it would create a never ending loop of read entry from log -> push to log -> read entry from log
pub fn initialize_from_wal(wal_file_path: &str, txd_count: Arc<RwLock<u32>>, vid_count: Arc<RwLock<u32>>,prev_lsn: Arc<RwLock<u64>>, current_transaction: Arc<RwLock<Transaction>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, ts_ord: Arc<RwLock<HashMap<u32, u32>>>) -> io::Result<()> {

    let mut file = File::open(wal_file_path)?;
    let mut newest_command = 0;

    loop {
        match bincode::deserialize_from::<&mut File, RecordStruct> (&mut file) {
            Ok(point) => {
                let command_type = point.command_type;
                newest_command = command_type;
                let mut command = create_command(point.payload, command_type);

                command = format!("{:?} 127.0.0.1:34254", command.trim_matches('"') ).replace("\"", "");

                match cli(command, Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn),Arc::clone(&current_transaction), Arc::clone(&new_node), None, Arc::clone(&all_addr), Arc::clone(&ts_ord), false) {
                    Ok(_) => {}
                    Err(e) => println!("WAL recovery error: {}", e),
                }

            },

            Err(err) => match *err {
                bincode::ErrorKind::Io(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }

                other => {
                    eprintln!("We got an error {:?}", other);
                }
            }
        }
    }

    if newest_command != 0x01 || newest_command != 0x02 {
        match cli(String::from("abort 127.0.0.1:34254"), Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn),Arc::clone(&current_transaction), Arc::clone(&new_node), None, Arc::clone(&all_addr), Arc::clone(&ts_ord), false) {
            Ok(_) => {}
            Err(e) => println!("WAL recovery error: {}", e),
        }
    }

    // Temporary solution -> Truncating modified log files
    match empty_file(wal_file_path) {
        Ok(_) => {}
        Err(e) => println!("File truncation error: {}", e),
    }

    Ok(())
}