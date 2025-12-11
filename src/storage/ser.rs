pub use std::io::Write;
use std::fs::{File, OpenOptions};
use std::io;
use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

pub fn serialize(node: Arc<RwLock<Node>>, serialized_file_path: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(serialized_file_path)?;

    writeln!(file, "[0]").expect("TODO: panic message");
    serialization(node, &mut file);
    Ok(())
}

pub fn serialization(node: Arc<RwLock<Node>>, file: &mut File) {
    let node_instance = node.read().unwrap();
    let l = node_instance.input.len();
    writeln!(file, "[{:X}]", node_instance.rank).expect("Error writing to file.");
    writeln!(file, "[{:X}]", l).expect("panic message");
    for i in 0..l {
        write!(file, "[{}]", node_instance.input[i].key).expect("panic message");
        let version_len = node_instance.input[i].version.len();
        writeln!(file, "[{}]", version_len).expect("panic message");
        for ver in &node_instance.input[i].version {
            write!(file, "[{}]", ver.xmin).expect("panic message");
            match ver.xmax {
                Some(xm) => {
                    write!(file, "[{}]", xm).expect("panic message");
                }
                None => {
                    write!(file, "[-]").expect("panic message");
                }
            }
            let value_len = ver.value.len();
            writeln!(file, "[{}]", value_len).expect("panic message");
            let x: Vec<char> = ver.value.chars().collect();
            write!(file, "{:?}", x).expect("panic message");
            writeln!(file, "").expect("panic message");
        }
    }
    writeln!(file, "[{:X}]", node_instance.children.len()).expect("panic message");
    if !node_instance.children.is_empty() {
        for i in 0..node_instance.input.len() + 1 {
            let z = Arc::clone(&node_instance.children[i]);
            serialization(z, file);
        }
    }
}
    
