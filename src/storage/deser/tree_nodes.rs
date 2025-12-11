use crate::btree::node::Items;

#[derive(Debug, Clone)]
pub struct KeyVersionNode {
    pub items: Vec<Items>,
    pub child_count: u32,
}
#[derive(Debug, Clone)]
pub struct HierarchicalNode {
    pub parent: KeyVersionNode,
    pub children: Vec<HierarchicalNode>,
}
