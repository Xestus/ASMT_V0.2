// fn get_key_version_node(vec) -> Vec<KeyVersionNode>

// fn get_hierarchical_node(root, &mut vec) -> HierarchicalNode

use crate::btree::node::{Items, Node};
use crate::MVCC::versions::Version;
use crate::storage::deser::num_or_str::*;
use crate::storage::deser::tree_nodes::{HierarchicalNode, KeyVersionNode};

pub fn get_key_version_node(vec: Vec<I32OrString>) -> Vec<KeyVersionNode> {
    let vector_len = vec.len();
    let mut count = 0;
    let mut no_of_keys_in_node = 0;
    let mut keys = 0;
    let mut version_of_single_key = 0;
    let mut dec_count_for_versions = -1;
    let mut to_set_version = false;
    let mut version_of_all_keys_in_same_node = 0;
    let mut xmin_vec: Vec<i32> = Vec::new();
    let mut xmax_vec: Vec<i32> = Vec::new();
    let mut values_vec: Vec<String> = Vec::new();
    let mut no_of_children = 0;
    let mut node_rank = 0;
    let mut vector_deserialized_items = Vec::new();
    let mut vector_deserialized = Vec::new();
    let mut internal_count = 0;
    let mut internal_count_activate = true;
    let mut number_of_keys_inspected = 0;
    let mut version_and_key_equal = 0;
    // Initial root done. Upto 19.
    for i in 0..vector_len {
        count += 1;
        if internal_count_activate {
            internal_count = count;
        }

        println!("------------------------------------------------");
        println!("{:?} \n count: {} \n no_of_keys_in_node: {} \n no_of_version_of_all_keys_in_same_node: {}", vec[i], count, no_of_keys_in_node, version_of_all_keys_in_same_node);
        println!(" dec_count_for_versions: {:?}", dec_count_for_versions);
        println!("version_of_single_key: {:?}", version_of_single_key);
        println!("number_of_keys_inspected: {:?}", number_of_keys_inspected);
        println!("--------------------------------------------------");
        if count == (3 + 5 * version_of_all_keys_in_same_node + no_of_keys_in_node + version_and_key_equal) && count != 3 && version_of_all_keys_in_same_node != version_of_single_key && no_of_keys_in_node > 0 && number_of_keys_inspected == no_of_keys_in_node {
            version_of_all_keys_in_same_node = 0;
            version_of_single_key = 0;
            no_of_keys_in_node = 0;
            println!("{:?}", vec[i]);
            no_of_children = vec[i].to_i32().unwrap();
            vector_deserialized.push(KeyVersionNode { child_count: no_of_children as u32, items: vector_deserialized_items.clone() });
            count = 1;
            println!("A");
            dec_count_for_versions = -1;
            // internal_count_activate += 1;
            number_of_keys_inspected = 0;
            version_and_key_equal = 0;
        }

        if to_set_version {
            version_of_single_key = vec[i].to_i32().unwrap();
            dec_count_for_versions = 4 * version_of_single_key + 1;
            version_of_all_keys_in_same_node += version_of_single_key;
            to_set_version = false;
            number_of_keys_inspected += 1;
            if number_of_keys_inspected == no_of_keys_in_node && version_of_all_keys_in_same_node == no_of_keys_in_node {
                println!("AAAAAAAAAAAAAAAAAAA");
                version_and_key_equal += 1;
            }
        }

        if dec_count_for_versions == 0 {
            println!("Key {:?}, {}", vec[i], count);
            keys = vec[i].to_i32().unwrap();
            to_set_version = true;
            dec_count_for_versions = -1;
        }

        if dec_count_for_versions > 0 && dec_count_for_versions <= 4 * version_of_single_key {
            if dec_count_for_versions % 4 == 0 {
                println!("XMIN {:?}", vec[i]);
                xmin_vec.push(vec[i].to_i32().unwrap());
            } else if (dec_count_for_versions + 1) % 4 == 0 {
                xmax_vec.push(vec[i].to_i32().unwrap());
                println!("XMAX {:?}", vec[i]);
            } else if (dec_count_for_versions + 3) % 4 == 0 {
                values_vec.push(vec[i].to_string().unwrap());
                println!("VALUE {:?}", vec[i]);
            }

            dec_count_for_versions -= 1;
        } else if dec_count_for_versions > 0 {
            dec_count_for_versions -= 1;
        }
        if dec_count_for_versions == 0 {
            let mut ver_vec: Vec<Version> = Vec::new();
            for j in 0..xmin_vec.len() {
                ver_vec.push(Version { value: values_vec[j].clone(), xmin: xmin_vec[j] as u32, xmax: Some(xmax_vec[j] as u32) });
            }
            values_vec.clear();
            xmin_vec.clear();
            xmax_vec.clear();

            vector_deserialized_items.push(Items { key: keys as u32, rank: node_rank as u32, version: ver_vec.clone() });

            println!("===================================");
            println!("vector_deserialized_items: {:?}", vector_deserialized_items);
            println!("===================================");
        }

        if count == 3 /*&& internal_count_activate == 1*/  {
            println!("{:?} {:?}", vec[i], vec[i - 1]);
            no_of_keys_in_node = vec[i].to_i32().unwrap();
            node_rank = vec[i - 1].to_i32().unwrap();

            dec_count_for_versions = 0;
        }
    }

    vector_deserialized
}

pub fn get_hierarchical_node(required_node: KeyVersionNode, node_vec: &mut Vec<KeyVersionNode>) -> HierarchicalNode {
    let mut x = HierarchicalNode {
        parent: required_node.clone(),
        children: Vec::new(),
    };
    if required_node.child_count > 0 {
        let mut i = 0;
        while i < node_vec.len() && required_node.child_count > x.children.len() as u32 {
            if required_node.items[0].rank + 1 == node_vec[i].items[0].rank {
                x.children.push(HierarchicalNode {
                    parent: node_vec[i].clone(),
                    children: Vec::new(),
                });
                node_vec.remove(i);
            } else {
                i += 1;
            }
        }
    }

    if x.parent.child_count > 0 {
        for i in 0..(x.children.len()) {
            let mut z;
            if x.children[i].parent.child_count != 0 {
                z = get_hierarchical_node(x.children[i].parent.clone(), node_vec);
                x.children.push(z);
            }
        }
    }
    x
}
