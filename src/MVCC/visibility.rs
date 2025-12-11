use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::MVCC::versions::Version;
use crate::transactions::transactions::*;

pub fn select_key(node: Arc<RwLock<Node>>, k: u32, last_txd: u32, current_txd: u32, status: Arc<RwLock<Transaction>>) -> Option<String> {
    // HAck: Looks inefficient, redo it later
    let mut result = Vec::new();
    match fetch_versions_for_key(node, k) {
        Some(version) => {
            result = version;
        }
        None => {}
    }
    if result.is_empty() {
        return None;
    }
    let status_read_guard = status.read().unwrap();

    let mut selected_keys = Vec::new();
    for i in 0..result.iter().len() {
        let result_max = result[i].xmax;
        let result_min = result[i].xmin;
        let min_status = status_read_guard.items.get(&result[i].xmin);
        // let min_status = status_read_guard.status.get(&result[i].xmin);

        let (mut visible_xmax, mut visible_xmin) = (false, false);

        match result_max {
            Some(xmax) => {
                if xmax >= last_txd {
                    visible_xmax = true;
                    // visible -- xmax >= current_txd_id
                }

                let max_status = status_read_guard.items.get(&xmax);

                if let Some(max_temp_status) = max_status {
                    if let TransactionStatus::Active = max_temp_status.status {
                        visible_xmax = true;
                        // visible -- xmax == ACTIVE
                    }
                }
            }
            None => {
                visible_xmax = true;
                // visible -- xmax == None

                if result_min == current_txd {
                    return Some(result[i].value.clone());
                }
            }
        }

        if (result_min == last_txd) {
            visible_xmin = true;
            // visible -- min == current_txd_id
        } else if let Some(min_temp_status) = min_status {
            if let TransactionStatus::Committed = min_temp_status.status
                && result_min < last_txd
            {
                visible_xmin = true;
                // visible
            }
        }

        if visible_xmax && visible_xmin {
            selected_keys.push(result[i].value.clone());
        }
    }

    if !selected_keys.is_empty() {
        return Some(selected_keys.last().unwrap().clone());
    }

    None
}

/// Searches selected key from a pre-defined B-Tree. If found, returns [`Option::Some(Items)`].
/// Iterative method was chosen rather than recursive as:
/// - The stack based function holds one lock at a time, preventing deadlocks.
/// - Prevents stack overflow and poor stability.
/// - Easier integration of concurrency + persistence.
///
/// # Working:
/// - Pushes the pre-defined B-Tree into `stack: Vec<Arc<Mutex<Node>>>`.
/// - Picks the last `Arc<Mutex<Node>>` from `stack`.
/// - Using iteration, if any key from the picked node matches the selected key, the [`Items`] is returned.
///     - [`Items`] is decently cheap to clone as its 32 bytes in size.
/// - Pushes the selected node via key range comparison to `stack` and repeats till the key is found.
/// - Returns a [`None`] if  the key isn't found.
///
/// # Condition:
/// - Every key is sorted by ascending as [`Node::sort_everything`] is invoked before invoking [`Node::fetch_versions_for_key`].
/// - Nodes are locked depth first, left to right.
/// - [`std::sync::PoisonError`] is a plausible error, handled temporarily by [`Result::unwrap_or_else`] assuming no corruption has occurred.
/// - [`Rc<RefCell<T>>`] isn't preferred because they're not for concurrent access across multiple threads.
/// - Root, Branch and Internal nodes, all of them will provide a valid result.
///
/// # Examples
/// ```rust
/// // Assume you have a B-tree with a key 1 on rank 2 with value "Woof".
///
/// let result = Node::key_position(new_node.clone(), required_key);
/// assert_eq!(result, Some(Items { key: 1, value: String::from("Woof"), rank: 2 }));
/// ```
///
/// ```rust
/// // Assume you have a B-Tree without the entered key.
///
/// let result = Node::key_position(new_node.clone(), required_key);
/// assert_eq!(result, None);
/// ```
///
/// # TODO + WARNING:
/// - THE SYSTEM CURRENTLY ISN'T CONCURRENT BUT IS CONCURRENCY IS THE NEXT FEATURE TO BE ADDED AFTER WRITE AHEAD LOGIN. PLEASE FORGIVE ME.
/// - CASES WITH READER/WRITER COLLISION WILL BE HANDLED WITH REPLACEMENT OF MUTEX WITH RWLOCK, DEPENDING UPON NEED.
pub fn fetch_versions_for_key(node: Arc<RwLock<Node>>, key: u32) -> Option<Vec<Version>> {
    let mut stack = Vec::new();
    stack.push(node);

    while let Some(self_node) = stack.pop() {
        let current = self_node.read().unwrap_or_else(|e| e.into_inner());

        for i in 0..current.input.len() {
            if current.input[i].key == key {
                return Some(current.input[i].version.clone());
            }
        }

        if !current.children.is_empty() {
            if key < current.input[0].key {
                stack.push(Arc::clone(&current.children[0]));
            } else if key > current.input[current.input.len() - 1].key {
                stack.push(Arc::clone(&current.children[current.children.len() - 1]));
            } else {
                for i in 0..current.input.len() - 1 {
                    if key > current.input[i].key && key < current.input[i + 1].key {
                        stack.push(Arc::clone(&current.children[i + 1]));
                    }
                }
            }
        }
    }
    None
}


