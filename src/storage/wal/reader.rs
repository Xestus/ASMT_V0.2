use std::fs::File;
use std::io;
use crate::storage::wal::log_utils::create_command;
use crate::storage::wal::record::RecordStruct;

/*pub fn get_uncommitted_transactions(wal_file_path: &str) -> io::Result<Vec<String>> {
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
}*/

pub fn get_uncommitted_transactions(wal_file_path: &str) -> io::Result<Vec<String>> {
    let mut file = File::open(wal_file_path)?;

    let mut uncommitted_strings = Vec::new();

    loop {
        match bincode::deserialize_from::<&mut File, RecordStruct> (&mut file) {
            Ok(point) => {
                if point.command_type == 0x01 || point.command_type == 0x02 {
                    uncommitted_strings.clear();
                } else {
                    let command = create_command(point.payload, point.command_type);
                    uncommitted_strings.push(command);
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

    Ok(uncommitted_strings)
}
