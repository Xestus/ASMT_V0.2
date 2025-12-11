// overflow_check, min_size_check, child_overflow

use std::sync::{Arc, RwLock};
use crate::btree::node::Node;
use crate::NODE_SIZE;

// To btree/scan
impl Node {
    /// A maintenance function responsible for checking overflows on designated nodes.
    /// The function iteratively checks children of the current Node only if the children exists and the node itself isn't overflowing.
    /// If the current node has its key count greater than maximum designated value, [`Node::split_nodes`] is invoked which splits overflowing node by relocating
    /// keys smaller and larger than middle keys as its children, while middle key stays at the same level.
    ///
    /// The function is expected to be called right and only after insertion.
    ///
    /// Panics if:
    /// - static `NODE_SIZE` is uninitialized.
    /// - The children mutex is poisoned when used as recursive functional parameter.
    /// But both the `.unwrap()` are safe, I think.
    ///
    /// - MutexGuard<Node> was used as both return and parameter because it allows reuse during recursion without relocking.
    pub fn overflow_check(root: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let cloned_root = Arc::clone(&root);
        let mut stack = Vec::new();
        let root_read = root.read().unwrap_or_else(|e| e.into_inner());

        if root_read.input.len() > *NODE_SIZE.get().unwrap() {
            drop(root_read);
            return Node::split_nodes(cloned_root);
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
        root
    }

    /// Checks for nodes violating the B-tree invariant `input.len() >= NODE_SIZE/2`
    ///
    /// # How does it happen?
    /// -  [`Node::split_nodes`] splits the overflowing (`input.len() > NODE_SIZE`) node that demotes non-middle key as children of middle key.
    /// - The number of middle key is always singular i.e. the node will only have 1 key which violates the B-Tree invariant.
    ///
    /// # Working:
    /// - The first iterator pushes indices of the current node's child that violates the B-Tree invariant of
    ///  `child.input.len() < NODE_SIZE/2 && child.rank > 1` to a temporary storage vector.
    ///     - Rank of root node is 1 and root node can have 1 key, `child.rank > 1` skips root node.
    /// - The second iterator reverses the iterator's direction and pushes the parent node and the invariant violator child to [`Node::propagate_up`]
    ///   that propagates the child & its own children to its parent.
    /// - The third iterator re-invokes the current function **if** any child of the current node has children.
    ///
    /// # Conditions:
    /// - `child_lock.input.len() < NODE_SIZE/2` still works if the maximum number of node count is either even or odd.
    ///     - The standard minimum key per node formula is:  `ceil((M + 1)/2) - 1` which gives the same result as `NODE_SIZE/2`.
    /// - Since [`Node::propagate_up`] removes children from the current node, the iteration is done in reverse
    ///       order to avoid issues with shifting child indices during removal.
    /// - The only error [`Node::propagate_up`] will return is a [`std::sync::PoisonError`] that is handled temporarily by [`Result::unwrap_or_else`].
    ///
    /// # TODO:
    /// - Implement safe locking with error handling (replace [`Result::unwrap_or_else`]).
    /// - Add poison propagation in case of locking errors.
    pub fn min_size_check(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let read_guard = self_node.read().unwrap();
        let k = read_guard.children.iter().enumerate();

        let mut indices_to_propagate = Vec::new();
        for (idx, child) in k {
            let child_read = child.read().unwrap_or_else(|e| e.into_inner());
            if child_read.input.len() < *NODE_SIZE.get().unwrap() / 2 && child_read.rank > 1 {
                indices_to_propagate.push(idx);
            }
        }

        drop(read_guard);

        for &idx in indices_to_propagate.iter().rev() {
            let child = {
                let read_guard = self_node.read().unwrap();
                Arc::clone(&read_guard.children[idx])
            };
            let meow = Arc::clone(&self_node);
            self_node = Node::propagate_up(meow, child);
        }

        let read_guard = self_node.read().unwrap();
        let y = &read_guard.children;

        for child in y {
            let child_children_empty = {
                let child_guard = child.read().unwrap_or_else(|e| e.into_inner());
                child_guard.children.is_empty()
            };
            if !child_children_empty {
                let k = child.clone();
                Node::min_size_check(k);
            }
        }

        drop(read_guard);
        self_node
    }

    /// Checks for nodes violating the B-tree invariant `children.len() == input.len() + 1`,
    /// which can occur after a split or merge operation.
    /// An example of under-flowing node with large child count.
    /// ```
    ///                                [754]
    ///    -----------------------------|---------------------------------------
    ///   /       |         |           |          |          |        |        \
    /// [7,9]  [410,480] [615,627]  [786, 809] [847, 879] [940,942] [365, 577] [839, 881]
    /// ```
    ///
    /// B-Tree must obey the rule which states: For every leaf nodes, number of parent keys + 1 == number of children node
    /// If the designated node fails the condition, the function `fix_child_count_mismatch` is invoked which fixes the failed condition by:
    /// - The violating node's keys are redistributed:
    ///     - Extract the first/last keys of all children to determine merge candidates.
    ///     - Nodes with overlapping key ranges are merged (e.g., `[365, 577]` and `[839, 881]`).
    /// - The first and last keys of the selected nodes are used as to identify nodes that span key ranges overlapping with others.
    ///     - Sentinel values ensure nodes with minimal/maximal keys are merged last,preserving tree order during rebalancing.
    /// - Remaining nodes are arranged as children based on the first and last keys of selected nodes.
    ///
    /// Variable `_unused` is safe because the guard is automatically dropped when the variable goes out of scope and the variable goes out of scope right after it is declared.
    ///
    /// There will be no deadlocks on iteration due to usage of variable `temporary_guard` which breaks
    /// **Circular wait** of **Coffman's conditions** and every thread will access children in the given order.
    ///
    /// Variable `temporary_guard` *can* be poisoned but the value of MutexGuard<Node> will be extracted by `unwrap_or_else(...)`.
    /// It's a patchy solution but stay till I add find better solution while redesigning system concurrent.
    pub fn child_overflow(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut stack = Vec::new();

        let self_instance = self_node
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if self_instance.input.len() + 1 != self_instance.children.len()
            && !self_instance.children.is_empty()
        {
            drop(self_instance);
            return Node::fix_child_count_mismatch(self_node);
        }

        let node_children = &self_instance.children;
        for child in node_children {
            stack.push(Arc::clone(child));
        }

        while let Some(node) = stack.pop() {
            let current_clone = Arc::clone(&node);
            let current_instance = node.read().unwrap_or_else(|poisoned| poisoned.into_inner());
            if current_instance.input.len() + 1 != current_instance.children.len()
                && !current_instance.children.is_empty()
            {
                drop(current_instance);
                let _unused = Node::fix_child_count_mismatch(current_clone);
            } else if !current_instance.children.is_empty() {
                let current_children = &current_instance.children;
                for child in current_children {
                    stack.push(Arc::clone(child));
                }
            }
        }
        drop(self_instance);
        self_node
    }

}