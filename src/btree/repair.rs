//split_nodes, propagate_up, fix_child_count_mismatch, rank_correction

use std::sync::{Arc, RwLock};
use crate::btree::node::Node;

// To btree/repair

impl Node {
    /// A private function exclusively invoked from `fn overflow_check` only if the selected node is overflowing i.e. The number of keys on the selected node
    /// exceeds maximum pre-defined threshold.
    ///
    /// The selected node is split by pushing the smaller and larger keys than the middle key as its children, while the middle key stays at the same level.
    /// There will be no change to pre-existing children nodes. The newly added child nodes will be placed on the last 2 indices on the field `children`.
    ///
    /// The splitting would create a temporary state of under-flowing node but quickly resolved by the recursive function `tree_integrity_check`
    /// that checks whether the node has number of keys lesser than minimum pre-defined threshold.
    /// The parent will node will have exactly 1 key and 2 new children.
    ///
    ///
    /// The nodes containing the smaller and larger keys will always have their rank as one more than their parent (Node with middle key).
    /// The field `rank` is modified twice, inside and after the loop because two struct containing field `rank`,
    /// Rank field of:
    /// - struct `Node` represents the rank of the Node that contains explicit number of keys.
    /// - struct `Items` represents rank of the individual key/value.
    ///
    /// (The key count of the node + 1)/2 is used to determine middle key, when the key count is:
    /// - Odd: The middle key splits the node into 2 equal half.
    /// - Even: The middle key splits the node into 2 half, where `number of keys larger than middle key - 1 = number of keys smaller than middle key`
    ///
    /// As the minimum possible designated maximum number of keys per node is 4, cases such as `.input.len()` being 0 or 1 is completely avoided.
    ///
    /// TODO: Edge Cases such as: Mutex for `struct_one` and `struct_two` being poisoned.
    ///
    pub fn split_nodes(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        self_node = Node::sort_main_nodes(self_node);
        let mut self_instance = self_node.write().unwrap_or_else(|e| e.into_inner()); // Mutable instance of self_node.

        let struct_one = Node::new(); // Holds keys smaller than middle key.
        let struct_two = Node::new(); // Holds keys larger than middle key.

        let items_size = self_instance.input.len();
        let breaking_point = (items_size + 1) / 2;
        let temp_storage = self_instance.clone();
        let mut i = 0;
        self_instance.input.clear();
        for count in 1..temp_storage.input.len() + 1 {
            if count == breaking_point {
                self_instance
                    .input
                    .push(temp_storage.input[count - 1].clone()); // Push the middle `Item` as sole parent.
            } else if count < breaking_point {
                struct_one
                    .write()
                    .unwrap()
                    .input
                    .push(temp_storage.input[count - 1].clone()); // Push the `Items` with keys smaller than middle key onto struct_one.
                struct_one.write().unwrap().input[count - 1].rank = temp_storage.rank + 1; // Set the key rank as parent node's rank + 1.
            } else if count > breaking_point {
                i = i + 1; // Variable "i" was used instead of count because `i` denotes the number of keys in struct_two.
                struct_two
                    .write()
                    .unwrap()
                    .input
                    .push(temp_storage.input[count - 1].clone()); // Push the `Items` with keys larger than middle key onto struct_two.
                struct_two.write().unwrap().input[i - 1].rank = temp_storage.rank + 1; // Set their key rank as parent node's rank + 1.
            }
        }

        // Set struct_one/two's node rank as parent's node rank + 1.
        struct_one.write().unwrap().rank = self_instance.rank + 1;
        struct_two.write().unwrap().rank = self_instance.rank + 1;

        self_instance.children.push(struct_one);
        self_instance.children.push(struct_two);

        drop(self_instance);
        self_node
    }

    /// Private function invoked from [`Node::min_size_check`], ensures all node meet the B-Tree invariant of `input.len() >= NODE_SIZE`.
    ///
    /// # Invocation:
    /// - Newly split node has singular key i.e. underfilled nodes (`input.len() < NODE_SIZE`)
    /// - [`Node::min_size_check`] determines the underfilled nodes and invokes the function.
    ///
    /// # Parameters:
    /// - `self_node`
    ///     - The parent Node where child's `Items` and children are passed to.
    ///     - Datatype of `MutexGuard<Node>` to avoid locking the same Mutex<Node> again inside the called function.
    ///     - `self_node` will have children and grandchildren as verified by [`Node::min_size_check`].
    /// - `child`
    ///     - The `Node` which holds `Items` and `Children` to be moved to `self_node`.
    /// - `child` will have children as a condition from [`Node::min_size_check`] and `child` is a single picked child of `self_node`, hence both of them will at least have a non-empty `input` field.
    ///
    /// # Working:
    /// - First iterator: Moves child's input to parent.
    ///     - Set the rank of child as parent's rank and is push to parent's input.
    ///     - Cloning is preferred as struct [`Items`] isn't expensive (28 bytes on 64-bit system)
    /// - Second iterator: Moves child's child to parent's child.
    ///     - Push to parent node's `children`.
    /// - Third iterator: Collect the indices of redundant `child` node from parent in `to_be_removed: Vec<usize>`
    /// - Fourth iterator: Remove the redundant `child` node from the parent using reverse of `to_be_removed: Vec<usize>` to prevent deadlocks.
    ///
    /// # Conditions:
    /// - `x.children[i].lock().unwrap()` is safe because:
    ///     - Only a single lock is held in an instance of time as there is no recursion or nested iterators resulting in not locking the same mutex *again*.
    ///     - No mutation of `x.children[i]` making it safe from iterator invalidation.
    /// - [`std::sync::PoisonError`] is a plausible error, handled temporarily by [`Result::unwrap_or_else`] assuming no corruption has occurred.
    /// - Duplicate keys aren't permittable by default and is handled to only allow unique keys to the B-Tree.
    /// - General guideline of locking nodes in ascending index order to prevent deadlocks.
    /// - Indices are collected first to avoid modifying `x.children` during iteration.
    ///
    /// # Diagram:
    ///
    /// Before (invalid):
    ///
    /// ```
    ///
    ///                 [230]
    ///                /--╨-\
    ///               /      \
    ///              /        \
    ///     [38, 55, 112]    [661]
    ///                      /-╨---\
    ///                     /       \
    ///                    /         \
    ///               [353, 513]  [670, 675]
    /// ```
    ///
    /// After (valid):
    ///
    /// ```
    ///                  [230, 661]
    ///                /-----╨-----\
    ///               /      |      \
    ///              /       |       \
    ///     [38, 55, 112] [353, 513] [670, 675]
    /// ```
    ///
    /// # ToDo: (ADD THEM BEFORE CONCURRENCY)
    /// - Replace [`Result::unwrap_or_else`] with `safe_lock<T>`
    /// - Propagate poisoning via `Result<MutexGuard<T>, TreeError>`.
    ///
    pub fn propagate_up(mut self_node: Arc<RwLock<Node>>, child: Arc<RwLock<Node>>, ) -> Arc<RwLock<Node>> {
        let self_read = self_node.read().unwrap();
        let mut child_write = child.write().unwrap();

        for child_input in &mut child_write.input {
            child_input.rank = self_read.rank;
        }
        drop(self_read);
        drop(child_write);

        let mut self_write = self_node.write().unwrap();
        let child_read = child.read().unwrap();

        let child_len = child_read.children.len();

        for child_push in child_read.input.clone() {
            self_write.input.push(child_push);
        }

        for i in 0..child_len {
            self_write.children.push(child_read.children[i].clone());
        }

        let conditional_key = child_read.input[0].key;

        self_write.children.retain(|child_arc| {
            let child_guard = child_arc.read().unwrap();
            child_guard.input[0].key != conditional_key
        });
        drop(self_write);

        self_node = Node::sort_main_nodes(self_node);
        self_node = Node::sort_children_nodes(self_node);

        self_node
    }
    
    /// A private function exclusively invoked from `fn tree_integrity_check`.
    ///
    /// Its invoked if the temporary B-Tree structure violates the fundamental B-Tree property:
    /// - For non-leaf nodes, `children.len() == input.len() + 1`.
    ///
    /// That occurs due to the internal node's key count exceeding the maximum designated size,
    ///  it causes the node to split, forcing its non-middle key to be added as two children.
    ///
    /// The first double for loop iteration is used to compare the first and the last keys of every node with the other to discover and place overlapping node
    /// (Node whose first key subceeds and last key exceeds at least one other node's first key and last key respectively) on the last 2 indices where the
    /// overlapping nodes are to be used as parent nodes for the rest of the children.
    ///
    /// It's a temporary brute-force solution with some unnecessary cloning.
    /// `Naive O(N²)` algorithm *is* inefficient in comparison to `O(N Log N)` but is used as a placeholder to be replaced with (maybe) Sweep Line Algorithm.
    ///
    /// Its tolerable now as the maximum number of keys per node is relatively small.
    ///
    /// The last 2 overlapping nodes are sorted by first key on ascending order by preloading the keys to `key_nodes`, sorting them and placing them back
    /// to original child node which prevents deadlocking during sorting.
    ///
    /// Only the last 2 children are sorted because the last two children are assumed to be the new parent for rest of the children.
    ///
    /// Other children are assumed to be sorted and satisfy the B-Tree ordering policy.
    ///
    /// `unwrap_or_else()` is a temporary duct taped-error handling to be replaced with a `safe_lock<T>` wrapper and/or integrity check with match whenever deemed necessary.
    ///
    ///
    /// .
    ///
    /// Before (invalid):
    /// ```text
    ///                                [754]
    ///    -----------------------------|---------------------------
    ///   /       |         |           |          |        |        \
    /// [7,9] [410,480] [615,627] [786,809] [847,879] [940,942] [365,577] [839,881]
    /// ```
    /// After (valid):
    /// ```text
    ///                                      [754]
    ///                        /---------------╨-----------\
    ///                       /                             \
    ///                      /                               \
    ///                 [365, 577]                         [839, 881]
    ///         /---------╨--------\                    /-----╨--------\
    ///        /          |         \                  /      |         \
    ///       /           |          \                /       |          \
    /// [7, 9, 331]  [410, 480]  [615, 627]     [786, 809] [847, 879]   [940, 942]
    /// ```
    /// # TODO:
    /// - Replace brute-force overlap detection with sweep-line (reduce from O(N²) to O(N log N)).
    /// - Implement safe locking with error handling (replace `unwrap_or_else`).
    /// - Add poison propagation in case of locking errors.
    pub fn fix_child_count_mismatch(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_instance = self_node
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let child_len = self_instance.children.len();

        for i in 0..child_len {
            // The children cannot be empty as only the nodes with children not empty can invoke the function (!self_instance.children.is_empty())
            let some_val = self_instance.children[i].read().unwrap().clone();
            let some_val_input = &some_val.input;
            let keys_primary_required =
                vec![some_val_input[0].key, some_val_input.last().unwrap().key];
            for j in 0..child_len {
                let some_other_val = self_instance.children[j].read().unwrap().clone();
                let some_other_val_input = &some_other_val.input;
                let keys_secondary_required = vec![
                    some_other_val_input[0].key,
                    some_other_val_input.last().unwrap().key,
                ];

                // Checks for overlapping node.
                if keys_primary_required[0] < keys_secondary_required[0]
                    && keys_primary_required[1] > keys_secondary_required[1]
                {
                    let k = Arc::new(RwLock::new(some_val));
                    self_instance.children.push(k); // Pushes the overlapping node to the last index.
                    self_instance.children.remove(i); // Removes the unnecessary overlapping node.
                    break;
                }
            }
        }

        let len = self_instance.children.len();
        if len >= 2 {
            // Extract keys plus original index
            let mut key_nodes: Vec<_> = self_instance.children[len - 2..]
                .iter()
                .map(|node| {
                    let guard = node.read().unwrap();
                    (guard.input[0].key, Arc::clone(node)) // clone Arc, not the Node
                })
                .collect();
            // Sort by key
            key_nodes.sort_by(|a, b| a.0.cmp(&b.0));

            // Put back into original children slice
            for (i, (_, node)) in key_nodes.into_iter().enumerate() {
                self_instance.children[len - 2 + i] = node;
            }
        }

        let mut parent_one_child_boundary = Vec::new();
        let mut parent_two_child_boundary = Vec::new();

        // Snippet placed inside a code block because `guard_parent_one` and `guard_parent_two` takes an immutable reference.
        {
            let guard_parent_one = self_instance.children[self_instance.children.len() - 2]
                .read()
                .unwrap_or_else(|p| p.into_inner());
            let guard_parent_two = self_instance.children[self_instance.children.len() - 1]
                .read()
                .unwrap_or_else(|p| p.into_inner());

            let new_parent_one_length = guard_parent_one.input.len();
            let new_parent_two_length = guard_parent_two.input.len();

            let guards = [&guard_parent_one, &guard_parent_two];

            // Assigns the minimum and maximum limit that the keys must fall to be placed as child for either of two new parents to parent_X_child_boundary.
            for i in 0..2 {
                let mut placeholder = Vec::new();
                // `i` being 1 and 2 assigns `new_parent_one_length` and `new_parent_two_length` to `k` respectively.
                let k = [new_parent_one_length, new_parent_two_length][i];
                // `i` being 1 and 2 assigns `guard_parent_one` and `guard_parent_two` to `k` respectively.
                let guard = guards[i];
                let require_child = vec![guard.input[0].key, guard.input[k - 1].key];

                if require_child[1] < self_instance.input.first().unwrap().key {
                    placeholder = vec![0, self_instance.input.first().unwrap().key]
                } else if require_child[0] > self_instance.input.last().unwrap().key {
                    placeholder = vec![self_instance.input.last().unwrap().key, u32::MAX]
                } else {
                    for j in 0..self_instance.input.len() - 1 {
                        if require_child[0] > self_instance.input[j].key
                            && require_child[1] < self_instance.input[j + 1].key
                        {
                            placeholder =
                                vec![self_instance.input[j].key, self_instance.input[j + 1].key]
                        }
                    }
                }
                // Assigns either of parent_X_child_boundary vector the minimum and maximum limit stored in a temporary placeholder according to value of `i`.
                match i {
                    0 => parent_one_child_boundary = placeholder,
                    1 => parent_two_child_boundary = placeholder,
                    _ => {}
                }
            }
        }
        let mut j = 0;

        // Remove the selected `Items` from child of selected node to its grandchild.
        for _i in 0..self_instance.children.len() - 2 {
            let k = self_instance.children[j].read().unwrap().clone();
            if k.input[0].key > parent_one_child_boundary[0]
                && k.input[k.input.len() - 1].key < parent_one_child_boundary[1]
            {
                self_instance.children[self_instance.children.len() - 2]
                    .write()
                    .unwrap()
                    .children
                    .push(Arc::new(RwLock::new(k)));
                self_instance.children.remove(j);
            } else if k.input[0].key > parent_two_child_boundary[0]
                && k.input[k.input.len() - 1].key < parent_two_child_boundary[1]
            {
                self_instance.children[self_instance.children.len() - 1]
                    .write()
                    .unwrap()
                    .children
                    .push(Arc::new(RwLock::new(k)));
                self_instance.children.remove(j);
            } else {
                j += 1;
            }
        }
        drop(self_instance);
        self_node
    }

    pub fn rank_correction(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let self_children_empty = {
            let self_read = self_node.read().unwrap();
            self_read.children.is_empty()
        };

        if self_children_empty {
            return self_node;
        }

        let mut self_write = self_node.write().unwrap();
        let child_rank = self_write.rank + 1;

        for child_arc in &mut self_write.children {
            {
                let mut child_write = child_arc.write().unwrap();
                child_write.rank = child_rank;

                for item in &mut child_write.input {
                    item.rank = child_rank;
                }
            }

            Node::rank_correction(Arc::clone(child_arc));
        }
        drop(self_write);

        self_node
    }

}