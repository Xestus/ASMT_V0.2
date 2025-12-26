use std::collections::HashMap;
use std::fs::{ OpenOptions};
use std::sync::{mpsc, Arc, RwLock };
use std::{io, thread};
use std::net::{TcpListener};

use ASMT::{NODE_SIZE};
use ASMT::engine::checkpoint::checkpoint;
use ASMT::engine::stream_processor::process_tcp_stream;
use ASMT::storage::io::is_file_empty;
use ASMT::btree::node::Node;
use ASMT::transactions::transactions::Transaction;
use ASMT::storage::wal::recovery::initialize_from_wal;

fn main() -> io::Result<()> {
    NODE_SIZE.set(4).expect("Failed to set size");
    let serialized_file_path = "/home/_merinh/RustroverProjects/ASMT_V0.2/example.txt";
    let wal_file_path = "/home/_merinh/RustroverProjects/ASMT_V0.2/WAL.txt";
    let mut new_node = Node::new();

    let current_transaction = Arc::new(RwLock::new(Transaction { items: HashMap::new(), ip_txd: HashMap::new() }));
    let all_address = Arc::new(RwLock::new(Vec::new()));

    match Node::deserialize(serialized_file_path) {
        Ok(node) =>  new_node = node,
        Err(e) => println!("{:?}", e),
    }

    let file = Arc::new(RwLock::new(OpenOptions::new()
        .append(true)
        .create(true)
        .open(wal_file_path)?));

    let cloned_node = Arc::clone(&new_node);
    let cloned_file = Arc::clone(&file);
    let cloned_addr = Arc::clone(&all_address);
    let cloned_transaction = Arc::clone(&current_transaction);

    let txd_count = Arc::new(RwLock::new(0));

    // if !is_file_empty(wal_file_path) { initialize_from_wal(wal_file_path, Arc::clone(&txd_count), Arc::clone(&current_transaction), Arc::clone(&file), Arc::clone(&new_node), Arc::clone(&all_address)); }

    let (tx, rx) = mpsc::channel();
    let t1 = thread::spawn(move || {
        while let Ok(_) = rx.recv() {
            checkpoint(Arc::clone(&cloned_node), serialized_file_path, wal_file_path, Arc::clone(&cloned_file), Arc::clone(&cloned_addr), Arc::clone(&cloned_transaction));
        }
    });

    let listener: TcpListener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on port 8080");
    for stream in listener.incoming() {
        let cloned_node = Arc::clone(&new_node);
        let cloned_file = Arc::clone(&file);
        let cloned_transaction = Arc::clone(&current_transaction);
        let cloned_txd_count = Arc::clone(&txd_count);
        let cloned_all_addr = Arc::clone(&all_address);
        let tx_clone = tx.clone();
        match stream {
            Ok(stream) => {
                thread::spawn(move || process_tcp_stream(stream, wal_file_path, cloned_txd_count, cloned_transaction, cloned_file, cloned_node, cloned_all_addr, tx_clone));
            }
            Err(e) => println!("Error: {}", e),
        }
    }
    drop(tx);
    t1.join().unwrap();

    Ok(())
}
