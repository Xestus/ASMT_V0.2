// fn get_node(deserialized_data: HierarchicalNode) -> Node

use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::storage::deser::tree_nodes::HierarchicalNode;
//fn deduplicate_children_recursive(node: Node) -> Node


pub fn deduplicate_children_recursive(mut self_node: Node) -> Node {
    let dup_child_len = self_node.children.len() / 2;

    let mut i = 0;
    while i < self_node.children.len() {
        let first_child = {
            let child_one_guard = self_node.children[i].read().unwrap();
            child_one_guard.input[0].clone()
        };
        let mut found_duplicate = false;

        for j in (i + 1)..self_node.children.len() {
            let second_child = {
                let child_two_guard = self_node.children[j].read().unwrap();
                child_two_guard.input[0].clone()
            };
            if first_child == second_child {
                self_node.children.remove(i);
                found_duplicate = true;
                break;
            }
        }

        if !found_duplicate {
            i += 1;
        }
    }

    for i in 0..dup_child_len {
        let child_guard = self_node.children[i].read().unwrap();
        let child_child_len = child_guard.children.len();
        let x = child_guard.clone();
        drop(child_guard);
        if child_child_len > 0 {
            self_node.children[i] =
                Arc::new(RwLock::new(deduplicate_children_recursive(x)));
        }
    }
    self_node
}

pub fn get_node(deserialized_data: HierarchicalNode) -> Node {
    let mut new_node = Node {
        input: Vec::new(),
        rank: 0,
        children: Vec::new(),
    };

    new_node.input = deserialized_data.parent.items.clone();
    new_node.rank = deserialized_data.parent.items[0].rank;

    if !deserialized_data.children.is_empty() {
        let child_len = deserialized_data.children.len();
        let mut child_vec = Vec::new();
        for j in 0..child_len {
            let k = Arc::new(RwLock::new(get_node(
                deserialized_data.children[j].clone(),
            )));
            child_vec.push(k);
        }
        new_node.children = child_vec;
    }

    new_node
}

