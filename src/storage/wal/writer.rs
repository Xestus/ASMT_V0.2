use std::io::{BufWriter, Seek, Write};
use std::fs::File;
use std::io;
use std::sync::{Arc, RwLock};
use crc32c::crc32c;
use crate::storage::wal::record::{Payload, RecordStruct};
use serde::{Serialize, Deserialize};

/*pub fn flush_to_wal(file: Arc<RwLock<File>>, args: Vec<&str>) -> io::Result<()> {
    let args = args.join(" ");

    let mut file_instance = file.write().unwrap();

    writeln!(file_instance, "{:?}", args).expect("TODO: panic message");
    file_instance.sync_all()?;

    Ok(())
}*/

pub fn flush_to_wal(args : String, payload: Option<Payload>, prev_lsn: u64) -> io::Result<u64> {
    let int_of_args = find_type(args.as_str());
    let payload_len = size_of_val(&payload) as u32;

    let mut record_struct_instance = RecordStruct {
        magic: 0x01,
        version: 1,
        command_type: int_of_args,
        total_len: size_of::<RecordStruct>() as u32,
        payload_len,
        crc32c: 0,
        lsn: 0,
        prev_lsn,
        payload
    };

    let bytes = bincode::serialize(&record_struct_instance).expect("Failed to serialize struct");
    let crc = crc32c::crc32c(&bytes);
    record_struct_instance.crc32c = crc;

    let mut file_instance = File::open("log.bin")?;
    
    let current_position = file_instance.stream_position()?;
    record_struct_instance.lsn = current_position;

    let mut writer = BufWriter::new(file_instance);
    bincode::serialize_into(&mut writer, &record_struct_instance);
    std::io::Write::flush(&mut writer)?;

    Ok(current_position)
}

fn find_type(args: &str) -> u8 {
    match args {
        "begin" => 0x00,
        "commit" => 0x01,
        "abort" => 0x02,
        "insert" => 0x03,
        "update" => 0x04,
        "delete" => 0x05,
        _ => 0xFF,
    }
}