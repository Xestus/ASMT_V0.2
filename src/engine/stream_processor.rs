use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use crate::btree::node::Node;
use crate::CHECKPOINT_COUNTER;
use crate::cli::cli::cli;
use crate::transactions::transactions::Transaction;

pub fn process_tcp_stream(mut stream: TcpStream, wal_file_path: &str, txd_count: Arc<RwLock<u32>>, current_transaction: Arc<RwLock<Transaction>>, file: Arc<RwLock<File>>, new_node: Arc<RwLock<Node>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>, tx: Sender<i32>) -> io::Result<()> {
    // In session project.
    // println!("Enter 'Help' for available commands & 'exit' to quit.");

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut buffer = String::new();

    loop {
        buffer.clear();

        match reader.read_line(&mut buffer) {
            Ok(0) => {
                println!("Client {} disconnected", stream.peer_addr()?);
                break;
            }

            Ok(_) => {
                let command = buffer.trim().to_string();

                let addr = stream.peer_addr()?;
                let command = format!("{} {}", command, addr);

                println!("Client {}", command);

                match cli(command, Arc::clone(&txd_count), Arc::clone(&current_transaction), Arc::clone(&file), Arc::clone(&new_node), Some(&stream), Arc::clone(&all_addr)) {
                    Ok(1) => continue,
                    Ok(2) => break,
                    Ok(3) => {
                        println!(":HI");
                        CHECKPOINT_COUNTER.store(100, Ordering::Relaxed);
                    }
                    Ok(_) => {}
                    Err(e) => println!("Error: {}", e),
                }

                let metadata = fs::metadata(wal_file_path)?;
                let size = metadata.len();

                if CHECKPOINT_COUNTER.load(Ordering::Relaxed) >= 100 || size >= 1024 {
                    println!("ZZZZ");
                    tx.send(1).unwrap();

                    return Ok(());
                    println!("Maximum WAL file size exceeded.");
                    CHECKPOINT_COUNTER.store(0, Ordering::Relaxed);
                }

                stream.write_all(b"\n")?;
            }

            Err(e) => {
                println!("Error reading from {}: {}", stream.peer_addr()?, e);
                break;
            }
        }
    }


    /*    loop {
            // In session project.
            // print!(">  ");
            io::stdout().flush()?;

            let mut cli_input = String::new();


            match io::stdin().read_line(&mut cli_input) {
                Ok(_) => {
                    match cli(cli_input, Arc::clone(&txd_count), Arc::clone(&current_transaction), Arc::clone(&file), Arc::clone(&new_node)) {
                        Ok(1) => continue,
                        Ok(2) => break,
                        Ok(3) => {
                            CHECKPOINT_COUNTER.store(100, Ordering::Relaxed);
                        }
                        Ok(_) => {}
                        Err(e) => println!("Error: {}", e),
                    }
                    let metadata = fs::metadata(wal_file_path)?;
                    let size = metadata.len();

                    if CHECKPOINT_COUNTER.load(Ordering::Relaxed) >= 100 && size >= 1024 {
                        println!("Maximum WAL file size exceeded.");
                        CHECKPOINT_COUNTER.store(0, Ordering::Relaxed);
                    }
                }
                Err(e) => println!("Invalid argument. Error: {:?}", e),
            }
        }
    */
    Ok(())
}
