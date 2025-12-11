use std::io::Write;
use std::fs::File;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use crate::btree::node::Node;
use crate::CHECKPOINT_COUNTER;
use crate::cli::parser::parse_string;
use crate::transactions::transactions::{Transaction, TransactionItems, TransactionStatus};
use crate::storage::wal::writer::flush_to_wal;
use crate::MVCC::visibility::{select_key, fetch_versions_for_key};

pub fn cli(cli_input: String, txd_count: Arc<RwLock<u32>>, current_transaction: Arc<RwLock<Transaction>>, file: Arc<RwLock<File>>, new_node: Arc<RwLock<Node>>, mut stream: Option<&TcpStream>, all_addr: Arc<RwLock<Vec<SocketAddr>>> ) -> io::Result<(u8)> {
    println!("{:?}", cli_input);

    let log_message = |message: &str|{
        if let Some(s) = stream {
            println!("ZZZ {}", cli_input);
            let mut s = s;
            match writeln!(s, "{}", message) {
                Ok(_) => {},
                Err(e) => println!("Error writing to stream: {}", e),
            }
        } else {
            println!("{}", message);
        }
    };

    // todo: temp clone.
    match parse_string(cli_input.clone()) {
        Ok((addr, args_string)) => {
            let args: Vec<&str> = args_string.iter().map(|s| s.as_str()).collect();

            let mut all_addr_write = all_addr.write().unwrap();
            all_addr_write.push(addr);

            if args.is_empty() { return Ok(1); }
            match args[0].to_lowercase().as_str() {
                "begin" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");

                        return Ok(1);
                    }

                    // Only new key if the addr has a committed status or is empty (1st time), or else ask to commit the active transaction.

                    flush_to_wal(Arc::clone(&file), args)?;
                    let mut mut_txd_count = txd_count.write().unwrap();
                    *mut_txd_count += 1;

                    {
                        let mut tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                tx.ip_txd.insert(addr, *mut_txd_count);
                                let item = tx.items.get(&x);
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Committed {
                                        tx.items.insert(*mut_txd_count, TransactionItems {status: TransactionStatus::Active, socket_addr: addr, last_txd: x });
                                    } else {
                                        log_message("Previous transaction is still active. Close it to start a new one.");
                                    }
                                }
                            }
                            None => {
                                tx.ip_txd.insert(addr, *mut_txd_count);
                                tx.items.insert(*mut_txd_count, TransactionItems {status: TransactionStatus::Active, socket_addr: addr, last_txd: 0 });

                            }
                        }
                    }
                    // current_transaction.write().unwrap().status.insert(*mut_txd_count, TransactionStatus::Active);
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
                                let item = tx.items.get(&x);
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Active {
                                        tx.items.insert(x, TransactionItems {status: TransactionStatus::Committed, socket_addr: addr, last_txd: x });
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

                    // let mut_txd_count = txd_count.read().unwrap();
                    // LAST_ACTIVE_TXD.store(*mut_txd_count as usize, Ordering::SeqCst);
                    // current_transaction.write().unwrap().status.insert(*mut_txd_count, TransactionStatus::Committed);
                    // current_transaction.write().unwrap().last_txd = mut_txd_count.clone();
                }

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
                                if let Some(item) = item {
                                    if item.status == TransactionStatus::Active {
                                        tx.items.insert(x, TransactionItems {status: TransactionStatus::Aborted, socket_addr: addr, last_txd: x });
                                    } else {
                                        println!("Active transaction not found. Commit failed.");
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

                    // let mut_txd_count = txd_count.read().unwrap();
                    // current_transaction.write().unwrap().status.insert(*mut_txd_count, TransactionStatus::Aborted);
                }

                "insert" => {
                    if args.len() != 4 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    flush_to_wal(Arc::clone(&file), args.clone())?;

                    let key = args[1].parse::<u32>().expect("Invalid argument");
                    let value = args[2].parse::<String>().expect("Invalid argument");
                    {
                        let tx = current_transaction.write().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let _ = Node::insert(Arc::clone(&new_node), key, value, x);
                            }
                            None => {
                                println!("Active transaction not found. Insert failed.");
                                return Ok(1);
                            }
                        }
                    }

                    // let mut_txd_count = txd_count.read().unwrap();
                    // let _ = Node::insert(Arc::clone(&new_node), key, value, *mut_txd_count);

                    CHECKPOINT_COUNTER.fetch_add(1, Ordering::SeqCst);
                }

                "update" => {
                    if args.len() != 4 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let key = args[1].parse::<u32>().expect("Invalid argument");
                    let value = args[2].parse::<String>().expect("Invalid argument");

                    {
                        let mut tx = current_transaction.read().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(x) => {
                                match Node::find_and_update_key_version(Arc::clone(&new_node), key, Some(value), *x) {
                                    Some(_) => flush_to_wal(Arc::clone(&file), args)?,
                                    None => log_message("Key not found"),
                                }
                            }
                            None => {
                                println!("Active transaction not found. Update failed.");
                                return Ok(1);
                            }
                        }

                    }

                    // let mut_txd_count = txd_count.read().unwrap();
                    // match Node::find_and_update_key_version(Arc::clone(&new_node), key, Some(value), *mut_txd_count, ) {
                    //     Some(_) => {
                    //         Node::flush_to_wal(Arc::clone(&file), args.clone())?;
                    //     }
                    //     None => log_message("Key not found"),
                    // }
                }

                "delete" => {
                    if args.len() != 3 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let key = args[1].parse::<u32>().expect("Invalid argument");

                    {
                        let mut tx = current_transaction.read().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(x) => {
                                match Node::find_and_update_key_version(Arc::clone(&new_node), key, None, *x) {
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


                    // let mut_txd_count = txd_count.read().unwrap();
                    // match Node::find_and_update_key_version(Arc::clone(&new_node), key, None, *mut_txd_count) {
                    //     Some(_) => {
                    //         Node::flush_to_wal(Arc::clone(&file), args.clone())?;
                    //     }
                    //     None => log_message("Key not found"),
                    // }
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
                        let mut tx = current_transaction.read().unwrap();

                        match tx.ip_txd.get(&addr) {
                            Some(&x) => {
                                let last_txd = tx.items.get(&x).unwrap().last_txd;

                                let messages ;
                                match select_key(Arc::clone(&new_node), key, last_txd, x, Arc::clone(&current_transaction)) {
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



                    // let current_txd_count = txd_count.read().unwrap();
                    // let last_txd = current_transaction.read().unwrap().last_txd;
                    // let messages ;
                    // match Node::select_key(Arc::clone(&new_node), key, last_txd, *current_txd_count, Arc::clone(&current_transaction)) {
                    //     Some(value) => {
                    //         messages = format!("Value: {:?}", value)
                    //     },
                    //     None =>  {
                    //         messages = String::from("Key not found")
                    //     },
                    // }
                    //
                    // log_message(messages.as_str());
                }

                "dump" => {
                    if args.len() != 3 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }
                    let key = args[1].parse::<u32>().expect("Invalid argument");

                    let mut messages= String::new();
                    match fetch_versions_for_key(Arc::clone(&new_node), key) {
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

                "tree" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let message = format!("{:?}", new_node.read().unwrap());
                    let message_2 = format!("{:?}", new_node.read().unwrap().print_tree());
                    println!("{:?}", new_node.read().unwrap().print_tree());
                    log_message(message.as_str());
                    log_message(message_2.as_str());
                }

                "stats" => {
                    if args.len() != 2 {
                        log_message("Invalid argument");
                        return Ok(1);
                    }

                    let message = format!("{:?}", new_node.read().unwrap().print_stats());
                    log_message(message.as_str());
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



    Ok(0)
}
