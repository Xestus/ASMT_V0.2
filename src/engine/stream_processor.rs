use std::fs::File;
use std::{fs, io};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use crate::btree::node::Node;
use crate::CHECKPOINT_COUNTER;
use crate::cli::cli::cli;
use crate::transactions::transactions::Transaction;

pub fn process_tcp_stream(mut stream: TcpStream, wal_file_path: &str, txd_count: Arc<RwLock<u32>>, vid_count: Arc<RwLock<u32>>, prev_lsn: Arc<RwLock<u64>>, current_transaction: Arc<RwLock<Transaction>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, ts_ord: Arc<RwLock<HashMap<u32, u32>>>, tx: Sender<i32>) -> io::Result<()> {
    // In session project.
    // println!("Enter 'Help' for available commands & 'exit' to quit.");

    stream.set_nonblocking(false)?;

    let mut buffer = Vec::with_capacity(8192);
    let mut temp_buf = [0u8; 1024];

    loop {
        buffer.clear();

        // match reader.read_until(b'\n', &mut buffer)
        match stream.read(&mut temp_buf) {
            Ok(0) => {
                println!("Client {} disconnected", stream.peer_addr()?);
                println!("Rollback all uncommitted changes");

                let addr = stream.peer_addr()?;
                let command = format!("{} {}", String::from("abort"), addr);

                match handle_cli_and_checkpoint(command, wal_file_path, &stream, Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn), Arc::clone(&current_transaction), Arc::clone(&new_node), Arc::clone(&all_addr), Arc::clone(&ts_ord),  &tx ) {
                    Ok(1) => continue,
                    Ok(2) => break,
                    Ok(_) => {}
                    Err(e) => println!("Error: {}", e),
                }
            }
            Ok(n) => {
                buffer.extend_from_slice(&temp_buf[..n]);

                // Process ALL complete commands in the buffer
                while let Some(cmd_end) = buffer.iter().position(|&b| b == b'\n') {
                    let result = String::from_utf8_lossy(&buffer[..=cmd_end]);
                    let command = result.to_string();

                    let addr = stream.peer_addr()?;
                    let command = format!("{} {}", command, addr);
                    
                    match handle_cli_and_checkpoint(command, wal_file_path, &stream, Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn), Arc::clone(&current_transaction), Arc::clone(&new_node), Arc::clone(&all_addr),Arc::clone(&ts_ord),  &tx ) {
                        Ok(1) => continue,
                        Ok(2) => break,
                        Ok(_) => {}
                        Err(e) => println!("Error: {}", e),
                    }

                    buffer.drain(0..=cmd_end);
                }

                if buffer.len() > 50000 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData, "Buffer overflow"
                    ));
                }

                if buffer.is_empty() && buffer.capacity() > 8192 {
                    buffer.shrink_to(8192);
                }

                if n == 0 {
                    println!("Client {} disconnected", stream.peer_addr()?);
                    println!("Rollback all uncommitted changes");
                }
            }

            Err(e) => {
                println!("Error reading from {}: {}", stream.peer_addr()?, e);
                break;
            }
        }
    }
    Ok(())
}

fn handle_cli_and_checkpoint(command: String,wal_file_path: &str, mut stream: &TcpStream, txd_count: Arc<RwLock<u32>>, vid_count: Arc<RwLock<u32>>, prev_lsn: Arc<RwLock<u64>> ,current_transaction: Arc<RwLock<Transaction>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, ts_ord: Arc<RwLock<HashMap<u32, u32>>> ,tx: &Sender<i32> ) -> io::Result<u8> {
    match cli(command, Arc::clone(&txd_count), Arc::clone(&vid_count), Arc::clone(&prev_lsn), Arc::clone(&current_transaction) , Arc::clone(&new_node), Some(&stream), Arc::clone(&all_addr), Arc::clone(&ts_ord)) {
        Ok(1) => return Ok(1), // Invalid argument
        Ok(2) => return Ok(2), // Exit
        Ok(3) => { // Checkpoint
            CHECKPOINT_COUNTER.store(100, Ordering::Relaxed);
        }
        Ok(_) => {}
        Err(e) => println!("Error: {}", e),
    }

    let metadata = fs::metadata(wal_file_path)?;
    let size = metadata.len();

    if CHECKPOINT_COUNTER.load(Ordering::Relaxed) >= 100 || size >= 4096 {
        tx.send(1).unwrap();
        println!("Maximum WAL file size exceeded.");
        CHECKPOINT_COUNTER.store(0, Ordering::Relaxed);
    }

    stream.write_all(b"\n")?;

    Ok(0)
}