use std::fs::{File, OpenOptions};
use std::{fs, io};
use std::io::{Read, Write};

pub fn read_file(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;

    File::create(file_path)?;

    Ok(contents)
}

pub fn empty_file(file_path: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(b"")?;

    Ok(())
}

pub fn is_file_empty(file_path: &str) -> bool {
    match fs::metadata(file_path) {
        Ok(metadata) => metadata.len() == 0,
        Err(_) => false,
    }
}