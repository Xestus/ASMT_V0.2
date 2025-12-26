use std::io;
use std::sync::{Arc, RwLock};
use crate::btree::node::{Items, Node};
use crate::MVCC::versions::{Version, VersionStatus};

impl Node {
    pub fn insert(self_node: Arc<RwLock<Node>>, k: u32, v: String, txn: u32) -> io::Result<()> {
        {
            let ver = Version { value: v.clone(), xmin: txn, xmax: None, version_status: VersionStatus::Active };
            match Node::find_and_update_key_version(Arc::clone(&self_node), k, Some(v), txn, false) {
                Some(_) => {
                    println!("Key already exists");
                    return Ok(());
                }
                None => {
                    let version = vec![ver.clone()];
                    Node::add_new_keys(Arc::clone(&self_node), Items { key: k, rank: 1, version }, );
                }
            }
        }

        Node::validate_after_mutation(self_node);

        Ok(())
    }

    /// # THIS IS A TEMPORARY HACK SOLUTION. IT'LL STAY THERE TILL I ADD AN ACTUAL THREAD SAFE FUNCTION.
    /// ## DO NOT TAKE THIS SERIOUSLY.
    /// ### :(
    pub fn find_and_update_key_version(node: Arc<RwLock<Node>>, key: u32, v: Option<String>, txn: u32, delete: bool) -> Option<()> {
        let mut write_guard = {
            let w1 = node.write();
            w1.unwrap_or_else(|poisoned| poisoned.into_inner())
        };
        for i in 0..write_guard.input.len() {
            if write_guard.input[i].key == key {
                let ver_count = write_guard.input[i].version.len();

                if delete {
                    let last_xmin = {
                        let x = &write_guard.input[i].version;
                        x[ver_count - 1].xmin
                    };

                    write_guard.input[i].version[ver_count - 1].xmax = Option::from(last_xmin);
                } else {
                    write_guard.input[i].version[ver_count - 1].xmax = Option::from(txn);
                }

                if ver_count >= 2 {
                    write_guard.input[i].version[ver_count - 2].xmax = None;
                }

                if let Some(value) = v {
                    let ver = Version {
                        value,
                        xmin: txn,
                        xmax: None,
                        version_status: VersionStatus::Active,
                    };
                    write_guard.input[i].version.push(ver);
                }
                return Some(());
            }
        }
        drop(write_guard);
        let read_guard = node.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !read_guard.children.is_empty() {
            if key < read_guard.input[0].key {
                let guard = Arc::clone(&read_guard.children[0]);
                drop(read_guard);
                return Node::find_and_update_key_version(guard, key, v, txn, delete);
            } else if key > read_guard.input[read_guard.input.len() - 1].key {
                let guard = Arc::clone(&read_guard.children.last().unwrap());
                drop(read_guard);
                return Node::find_and_update_key_version(guard, key, v, txn, delete);
            } else {
                for i in 0..read_guard.input.len() - 1 {
                    if key > read_guard.input[i].key && key < read_guard.input[i + 1].key {
                        let guard = Arc::clone(&read_guard.children[i + 1]);
                        drop(read_guard);
                        return Node::find_and_update_key_version(guard, key, v, txn, delete);
                    }
                }
            }
        }
        None
    }


    fn add_new_keys(self_node: Arc<RwLock<Node>>, mut x: Items) {
        let self_instance = &mut self_node.write().unwrap();
        if self_instance.children.is_empty() {
            self_instance.input.push(x.clone());
        } else {
            if x.key < self_instance.input[0].key {
                if !self_instance.children.is_empty() {
                    Node::add_new_keys(Arc::clone(&self_instance.children[0]), x);
                } else {
                    x.rank = self_instance.input[0].rank;
                    self_instance.input.push(x);
                }
            } else if x.key > self_instance.input[self_instance.input.len() - 1].key {
                if !self_instance.children.is_empty() {
                    Node::add_new_keys(
                        Arc::clone(&self_instance.children[self_instance.children.len() - 1]),
                        x,
                    );
                } else {
                    x.rank = self_instance.input[0].rank;
                    self_instance.input.push(x);
                }
            } else {
                for i in 0..self_instance.input.len() - 1 {
                    if x.key > self_instance.input[i].key && x.key < self_instance.input[i + 1].key
                    {
                        if !self_instance.children.is_empty() {
                            Node::add_new_keys(
                                Arc::clone(&self_instance.children[i + 1]),
                                x.clone(),
                            );
                        } else {
                            x.rank = self_instance.input[0].rank;
                            self_instance.input.push(x.clone());
                        }
                    }
                }
            }
        }
    }
}