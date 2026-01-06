use std::fs::File;
use std::io;
use crate::storage::wal::log_utils::create_command;
use crate::storage::wal::record::RecordStruct;

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
