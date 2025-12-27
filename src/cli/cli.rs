use std::io::Write;
use std::fs::File;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use crate::btree::node::Node;
use crate::CHECKPOINT_COUNTER;
use crate::cli::parser::parse_string;
use crate::MVCC::snapshot::snapshot;
use crate::transactions::transactions::{Transaction, TransactionItems, TransactionStatus};
use crate::transactions::manager::get_all_active_transaction;
use crate::storage::wal::writer::flush_to_wal;
use crate::MVCC::visibility::{select_key, modified_key_check, fetch_version_vec_for_key, commit_abort_handler};

pub fn cli(cli_input: String, txd_count: Arc<RwLock<u32>>, current_transaction: Arc<RwLock<Transaction>>, file: Arc<RwLock<File>>, new_node: Arc<RwLock<Node>>, stream: Option<&TcpStream>, all_addr: Arc<RwLock<Vec<SocketAddr>>> ) -> io::Result<u8> {
    println!("{:?}", cli_input);

    let log_message = |message: &str|{
        if let Some(s) = stream {
            let mut s = s;
            match writeln!(s, "{}", message) {
                Ok(_) => {},
                Err(e) => println!("Error writing to stream: {}", e),
            }
        } else {
            println!("{}", message);
        }
    };

    match parse_string(cli_input) {
        Ok((addr, args_string)) => {
            let args: Vec<&str> = args_string.iter().map(|s| s.as_str()).collect();

            let mut all_addr_write = all_addr.write().unwrap();
            all_addr_write.push(addr);
            drop(all_addr_write);

            if args.is_empty() { return Ok(1); }
            match args[0].to_lowercase().as_str() {
                "begin" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");

                        return Ok(1);
                    }

                    flush_to_wal(Arc::clone(&file), args)?;
                    let mut mut_txd_count = txd_count.write().unwrap();
                    *mut_txd_count += 1;

                    {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let item = tx.items.get(&x);
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Committed || item.status == TransactionStatus::Aborted {
                                        tx.ip_txd.insert(addr, *mut_txd_count);
                                        tx.items.insert(*mut_txd_count, TransactionItems {status: TransactionStatus::Active, socket_addr: addr, last_txd: x, modified_keys: Vec::new() });
                                        tx.items.remove(&x);
                                    } else {
                                        *mut_txd_count -= 1;
                                        log_message("Previous transaction is still active. Close it to start a new one.");
                                    }
                                }
                            }
                            None => {
                                tx.ip_txd.insert(addr, *mut_txd_count);
                                tx.items.insert(*mut_txd_count, TransactionItems {status: TransactionStatus::Active, socket_addr: addr, last_txd: 0, modified_keys: Vec::new() });

                            }
                        }
                    }
                }

                "commit" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    flush_to_wal(Arc::clone(&file), args)?;

                    {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let item = &mut tx.items.get(&x);
                                let mut modified_key_vec = Vec::new();
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Active {
                                        if let Some(items) = tx.items.get_mut(&x) {
                                            items.status = TransactionStatus::Committed;
                                            modified_key_vec = items.modified_keys.clone();
                                        }
                                        drop(tx);
                                        for j in modified_key_vec.iter() {
                                            commit_abort_handler(Arc::clone(&new_node), *j, true);
                                        }
                                    } else {
                                        println!("Active transaction not found. Commit failed.");
                                        return Ok(1);
                                    }
                                }

                            }
                            None => {
                                println!("Active transaction not found. Commit failed.");
                                return Ok(1);
                            }
                        }
                    }
                }

                // remove the new version when "abort"
                "abort" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }
                    flush_to_wal(Arc::clone(&file), args)?;

                    {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let item = tx.items.get(&x);
                                let mut modified_key_vec = Vec::new();
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Active {
                                        if let Some(items) = tx.items.get_mut(&x) {
                                            items.status = TransactionStatus::Aborted;
                                            modified_key_vec = items.modified_keys.clone();
                                        }
                                        drop(tx);
                                        for j in modified_key_vec.iter() {
                                            commit_abort_handler(Arc::clone(&new_node), *j, false);
                                        }
                                        println!("B");

                                    } else {
                                        println!("Active transaction not found. Abort failed.");
                                        return Ok(1);
                                    }
                                }
                            },
                            None => {
                                println!("Active transaction not found. Abort failed.");
                                return Ok(1);
                            }
                        }
                    }
                }

                "insert" => {
                    if args.len() != 4 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    flush_to_wal(Arc::clone(&file), args.clone())?;

                    let key = args[1].parse::<u32>().expect("Invalid argument");
                    let value = args[2].parse::<String>().expect("Invalid argument");

                    let active_txd_vec = get_all_active_transaction(Arc::clone(&current_transaction), all_addr);

                    let txd = current_transaction.read().unwrap().ip_txd.get(&addr).unwrap().clone();
                    let y = modified_key_check(active_txd_vec, key, txd, Arc::clone(&current_transaction) );

                    if y {
                        println!("The key you're trying to insert has already been updated by another client.")
                    } else {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let _ = Node::insert(Arc::clone(&new_node), key, value, x);
                                if let Some(item) = tx.items.get_mut(&x) {
                                    item.modified_keys.push(key);
                                }
                            }
                            None => {
                                println!("Active transaction not found. Insert failed.");
                                return Ok(1);
                            }
                        }
                        CHECKPOINT_COUNTER.fetch_add(1, Ordering::SeqCst);
                    }
                }

                "update" => {
                    if args.len() != 4 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let key = args[1].parse::<u32>().expect("Invalid argument");
                    let value = args[2].parse::<String>().expect("Invalid argument");

                    let active_txd_vec = get_all_active_transaction(Arc::clone(&current_transaction), all_addr);

                    let txd = current_transaction.read().unwrap().ip_txd.get(&addr).unwrap().clone();
                    let y = modified_key_check(active_txd_vec, key, txd, Arc::clone(&current_transaction) );

                    if y {
                        println!("The key you're trying to update has already been updated by another client.")
                    } else {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                match Node::find_and_update_key_version(Arc::clone(&new_node), key, Some(value), x, false) {
                                    Some(_) => flush_to_wal(Arc::clone(&file), args)?,
                                    None => log_message("Key not found"),
                                }

                                if let Some(item) = tx.items.get_mut(&x) {
                                    item.modified_keys.push(key);
                                }
                            }
                            None => {
                                println!("Active transaction not found. Update failed.");
                                return Ok(1);
                            }
                        }
                    }
                }

                "delete" => {
                    if args.len() != 3 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let key = args[1].parse::<u32>().expect("Invalid argument");

                    {
                        let tx = current_transaction.read().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(x) => {
                                match Node::find_and_update_key_version(Arc::clone(&new_node), key, None, *x, true) {
                                    Some(_) => {
                                        flush_to_wal(Arc::clone(&file), args.clone())?;
                                    }
                                    None => log_message("Key not found"),
                                }
                            }
                            None => {
                                println!("Active transaction not found. Update failed.");
                                return Ok(1);
                            }
                        }
                    }
                }

                "checkpoint" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    println!("HELLO");
                    return Ok(3);
                }

                "select" => {
                    if args.len() != 3 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }
                    let key = args[1].parse::<u32>().expect("Invalid argument");

                    {
                        let tx = current_transaction.read().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                // let last_txd = tx.items.get(&x).unwrap().last_txd;

                                let messages ;
                                match select_key(Arc::clone(&new_node), key, *txd_count.read().unwrap(), x, Arc::clone(&current_transaction)) {
                                    Some(value) => {
                                        messages = format!("Value: {:?}", value)
                                    },
                                    None =>  {
                                        messages = String::from("Key not found")
                                    },
                                }

                                log_message(messages.as_str());

                            }
                            None => {}
                        }
                    }
                }

                "dump" => {
                    if args.len() != 3 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }
                    let key = args[1].parse::<u32>().expect("Invalid argument");

                    let mut messages= String::new();
                    match fetch_version_vec_for_key(Arc::clone(&new_node), key) {
                        Some(value) => {
                            let mut message_vector = Vec::new();

                            for i in value.iter() {
                                let k ;
                                if let Some(xmax) = i.xmax {
                                    k = format!("Value: {:?} [xmin: {} -- xmax: {}]", i.value, i.xmin, xmax);
                                } else {
                                    k = format!("Value: {:?} [xmin: {} -- xmax: ∞]", i.value, i.xmin);
                                }

                                message_vector.push(k);
                            }

                            messages = message_vector.join(" ");
                        }

                        None => {
                            messages = String::from("Key not found")
                        },
                    }

                    log_message(messages.as_str());
                }


                // TODO: FIX TREE DISPLAY
                "tree" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    {
                        let tx = current_transaction.read().unwrap();
                        let duplicate_node = new_node.clone();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let tree_node = snapshot(duplicate_node, Some(x));
                                println!("{:?}", tree_node.read().unwrap().print_tree());
                            }
                            None => {}
                        }
                    }
                }

                "stats" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    {
                        let tx = current_transaction.read().unwrap();
                        let duplicate_node = new_node.clone();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let tree_node = snapshot(duplicate_node, Some(x));
                                println!("{:?}", tree_node.read().unwrap().print_stats());
                            }
                            None => {}
                        }
                    }
                }

                "help" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let messages = {
                        "insert <key> <value>  - Insert a key-value pair\n
                 update <key> <value>  - Update a key-value pair\n
                 select <key>          - Get the visible value for the key\n
                 dump <key>            - Get all the values for the key\n
                 delete <key>          - Delete a key\n
                 begin                 - Start a cycle\n
                 commit                - Push a new version of the key\n
                 abort                 - Abort the current cycle\n
                 tree                  - Show B-Tree in ASCII art form\n
                 stats                 - Show B-Tree Stats\n
                 help                  - List out all the commands\n
                 exit                  - Exit the program"
                    };

                    log_message(messages);
                }

                // Make "exit" quit the client, not the server
                "exit" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    log_message("Invalid argument");
                    return Ok(2);
                }

                _ => {

                    let message = format!("Unknown command: {}. Type 'help' for available commands.", args[0]);
                    log_message(message.as_str());
                },
            }

        }

        Err(e) => println!("Parse Error: {}", e),
    }

    println!("--------------------------------------------------");
    println!("{:#?}", current_transaction.read().unwrap());
    println!("--------------------------------------------------");


    Ok(0)
}