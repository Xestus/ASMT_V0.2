use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

impl Node {
    pub fn validate_after_mutation(mut node: Arc<RwLock<Node>>) {
        node = Node::overflow_check(node);
        node = Node::min_size_check(node);
        node = Node::sort_main_nodes(node);
        node = Node::sort_children_nodes(node);

        node = Node::child_overflow(node);

        node = Node::min_size_check(node);

        node = Node::overflow_check(node);

        node = Node::min_size_check(node);
        node = Node::sort_main_nodes(node);
        node = Node::child_overflow(node);
        node = Node::rank_correction(node);
        node = Node::sort_everything(node);

        node = Node::overflow_check(node);
        node = Node::min_size_check(node);
        node = Node::child_overflow(node);
        node = Node::rank_correction(node);
        node = Node::sort_everything(node);
    }
}