use std::io;
use std::sync::atomic::Ordering;
use crate::LAST_ACTIVE_TXD;
use crate::storage::io::{empty_file, read_file};

pub fn get_uncommitted_transactions(wal_file_path: &str) -> io::Result<Vec<String>> {
    let mut uncommitted_strings = Vec::new();

    match read_file(wal_file_path) {
        Ok(metadata) => {
            match empty_file(wal_file_path) {
                Ok(_) => {}
                Err(e) => {
                    println!("File truncation error: {}", e);
                }
            }

            let mut commit_count = 0;

            for line in metadata.lines() {
                let items = line.replace("\"", "");

                println!("{:?}", items);

                if commit_count >= LAST_ACTIVE_TXD.load(Ordering::SeqCst) {
                    uncommitted_strings.push(items.clone());
                }
                {
                    if items.to_lowercase().contains("commit") {
                        commit_count += 1;
                    }
                }
            }
        }
        Err(e) => {
            println!("File read error: {}", e);
        }
    }

    Ok(uncommitted_strings)
}
