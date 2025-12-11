use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

pub fn remove_dead_version(node: Arc<RwLock<Node>>, oldest_active_txd: u32) {
    let mut stack = Vec::new();

    retain_active_txd(Arc::clone(&node), oldest_active_txd);

    let node_read = node.read().unwrap_or_else(|e| e.into_inner());
    let root_children = &node_read.children;
    for child in root_children {
        stack.push(Arc::clone(child));
    }
    drop(node_read);

    while let Some(node) = stack.pop() {

        retain_active_txd(Arc::clone(&node), oldest_active_txd);

        let node_read = node.read().unwrap_or_else(|e| e.into_inner());

        if !node_read.children.is_empty() {
            let current_children = &node_read.children;
            for child in current_children {
                stack.push(Arc::clone(child));
            }
        }
    }
}

pub fn retain_active_txd(node: Arc<RwLock<Node>>, oldest_active_txd: u32) {
    let mut current_write = node.write().unwrap_or_else(|e| {
        eprintln!("Error: {:?}", e);
        e.into_inner()
    });
    for i in &mut current_write.input {
        i.version.retain(|f| {
            match f.xmax {
                Some(x_max) => {
                    if x_max < oldest_active_txd {
                        return false;
                    }
                    return true;
                }

                None => true,
            }
        })
    }
}