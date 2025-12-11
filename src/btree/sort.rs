use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

impl Node {
    pub fn sort_children_nodes(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_write = self_node.write().unwrap();
        self_write.children.sort_by(|a, b| {
            a.read().unwrap().input[0]
                .key
                .cmp(&b.read().unwrap().input[0].key)
        });
        drop(self_write);
        self_node
    }
    pub fn sort_main_nodes(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_write = self_node.write().unwrap();
        self_write.input.sort_by(|a, b| a.key.cmp(&b.key));
        drop(self_write);
        self_node
    }
    pub fn sort_everything(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        self_node = Node::sort_main_nodes(self_node);
        self_node = Node::sort_children_nodes(self_node);

        let self_read = self_node.write().unwrap();
        let children = self_read.children.clone();

        for child in children {
            Node::sort_everything(child);
        }

        drop(self_read);
        self_node
    }
}
