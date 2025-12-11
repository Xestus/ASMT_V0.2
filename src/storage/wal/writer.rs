use std::io::Write;
use std::fs::File;
use std::io;
use std::sync::{Arc, RwLock};

pub fn flush_to_wal(file: Arc<RwLock<File>>, args: Vec<&str>) -> io::Result<()> {
    let args = args.join(" ");

    let mut file_instance = file.write().unwrap();

    writeln!(file_instance, "{:?}", args).expect("TODO: panic message");
    file_instance.sync_all()?;

    Ok(())
}
