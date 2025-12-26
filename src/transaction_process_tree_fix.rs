use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::NODE_SIZE;
use crate::transactions::transactions::{Transaction, TransactionStatus};

/*fn transaction_stuffs(transaction: Arc<RwLock<Transaction>>, current_txd: Vec<u32>, node: Arc<RwLock<Node>>) {

    let x = transaction.read().unwrap();
    
    let mut active_txd = Vec::new();
    
    for i in current_txd {
        if x.items.get(&i).unwrap().status == TransactionStatus::Active {
            active_txd.push(i);
        }
    }
    

    let cloned_node = Arc::clone(&node);
    let mut stack = Vec::new();
    let root_read = node.read().unwrap_or_else(|e| e.into_inner());
    
    
    for i in root_read.input.iter() {
        for j in i.version.iter() {
            for k in active_txd.iter() {
                if j.xmax >= Some(*k) {
                    let ss = j.xmax;
                    j.xmax = None;
                }
                
                if j.xmin >= *k {
                    
                }

            }
            
        }
    }

    let root_children = &root_read.children;
    for child in root_children {
        stack.push(Arc::clone(child));
    }

    while let Some(node) = stack.pop() {
        let current_clone = Arc::clone(&node);
        let current_read = node.read().unwrap_or_else(|poisoned| poisoned.into_inner());

        if current_read.input.len() > *NODE_SIZE.get().unwrap() {
            drop(current_read);
            let _unused = Node::split_nodes(current_clone);
        } else if !current_read.children.is_empty() {
            let current_children = &current_read.children;
            for child in current_children {
                stack.push(Arc::clone(child));
            }
        }
    }

    drop(root_read);

}*/