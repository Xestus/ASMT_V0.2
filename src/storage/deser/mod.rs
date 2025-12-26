use std::io;
use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

pub mod tree_nodes;
pub mod num_or_str;
pub mod raw;
pub mod parse;
pub mod reconstruct;

use crate::storage::io::is_file_empty;

impl Node {
    pub fn deserialize(serialized_file_path: &str) -> io::Result<Arc<RwLock<Node>>> {
        if is_file_empty(serialized_file_path) {
            return Ok(Node::new());
        }

        let required_node = match raw::get_serialized_file_data(serialized_file_path) {

            Ok (vec) => {
                let mut key_version_node = parse::get_key_version_node(vec);

                let required_node = key_version_node[0].clone();
                let hierarchical_node_instance = parse::get_hierarchical_node(required_node, &mut key_version_node);

                let mut constructed_node = reconstruct::get_node(hierarchical_node_instance);
                constructed_node = reconstruct::deduplicate_children_recursive(constructed_node);

                let constructed_node = Arc::new(RwLock::new(constructed_node));
                println!("{:?}", constructed_node.read().unwrap().print_tree());

                constructed_node
            }
            Err(e) => {
                return Err(e);
            }
        };
        Ok(required_node)
    }
}
