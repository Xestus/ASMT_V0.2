use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use crate::btree::node::Node;
use crate::LAST_ACTIVE_TXD;
pub fn fetch_serializable_btree(node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
    let xmax_threshold = LAST_ACTIVE_TXD.load(Ordering::SeqCst) as u32;

    {
        let mut node_guard = node.write().unwrap();

        for input in &mut node_guard.input {
            let mut to_be_removed = Vec::new();
            for i in 0..input.version.len() {
                if let Some(xmax) = input.version[i].xmax {
                    if xmax > xmax_threshold {
                        input.version[i].xmax = None;
                    }
                }

                if input.version[i].xmin > xmax_threshold {
                    to_be_removed.push(i);
                }
            }

            for i in to_be_removed.iter().rev() {
                input.version.remove(*i);
            }
        }
    }

    let self_children_empty = {
        let self_read = node.read().unwrap();
        self_read.children.is_empty()
    };

    if self_children_empty {
        return node;
    }

    let node_guard = node.read().unwrap();

    for child_arc in &node_guard.children {
        let clone = Arc::clone(child_arc);
        fetch_serializable_btree(clone);
    }

    drop(node_guard);

    node
}

