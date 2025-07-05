use std::cell::RefCell;
use crate::rand::Rng;
use std::io::{BufRead, BufReader, Write};
extern crate rand;

use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use once_cell::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs::{File, OpenOptions};
use std::rc::Rc;
use regex::Regex;

static NODE_SIZE: OnceCell<usize> = OnceCell::new();

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
struct Items {
    key: u32,
    value: String,
    rank: u32,
}
#[derive(Debug, Clone)]
struct Node {
    input: Vec<Items>,
    rank: u32,
    children: Vec<Arc<Mutex<Node>>>,
}


#[derive(Debug)]
enum U32OrString {
    Num(u32),
    Str(String),
}

#[derive(Debug, Clone)]
struct DeserializedNode {
    items: Vec<Items>,
    child_count: u32,
}
#[derive(Debug, Clone)]
struct UltraDeserialized {
    parent: DeserializedNode,
    children: Vec<UltraDeserialized>,
}

impl Node {
    
    /// The function creates a new empty node for a B-tree during initialization and operations such as splitting.
    /// The default value of field `rank` is 1 as:
    /// - A new B-Tree always starts with an empty Node with `rank: 1`.
    /// - Any Node that isn't root node has it's rank default to `parent's rank + 1`.
    ///
    /// The fields `input` and `children` are initialized as empty Vector as:
    /// - A new B-Tree's root node will always have 0 items and 0 children.
    /// - Any new Node from splitting would have its input and children derived from its predecessor Node.
    ///
    /// The new Node is wrapped with Arc and Mutex as Mutex allows Thread-safe mutations and Arc allows Mutex to be shared across threads.
    /// As the B-Tree will eventually scale up to concurrency, Arc<Mutex<T>> helps in future proofing the concept.
    ///
    fn new() -> Arc<Mutex<Node>> {
        let instance = Arc::new(Mutex::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
        }));
        instance
    }

    // Insert the K-V into the empty node.
    // Todo: Understand why i had to call every function 3 times for correct functioning.
    fn insert(self_node: &mut Arc<Mutex<Node>>, k: u32, v: String) {
        let mut z = self_node.try_lock().unwrap();
        let rank = z.rank;
        if !z.children.is_empty() {
            z.add_child_key(Items {key: k, value: v.clone(), rank});
        }
        else {
            z.input.push(Items {key: k, value: v, rank});
        }
        
        // Every function takes variable z with datatype MutexGuard<T> because it's the default form after .lock().unwrap() on Arc<Mutex<T>>.
        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z.sort_main_nodes();
        z.sort_children_nodes();
        z = Node::tree_integrity_check(z);
        z = Node::min_size_check(z);
        
        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z.sort_main_nodes();
        z = Node::tree_integrity_check(z);
        z = Node::rank_correction(z);
        z.sort_everything();
        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z = Node::tree_integrity_check(z);
        z = Node::rank_correction(z);
        z.sort_everything();

        let k = RefCell::new("MEOW");
    }
    
    /// A maintenance function responsible for checking overflows on designated nodes.
    /// The function recursively check children of the current Node only if the children exists and the node itself isn't overflowing.
    /// If the current node has its key count greater than maximum designated value, a function "split_node" is invoked which splits overflowing node by relocating
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
    fn overflow_check(mut root: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut stack = Vec::new();

        if root.input.len() > *NODE_SIZE.get().unwrap() {
            return Node::split_nodes(root);
        }

        for child in &root.children {
            stack.push(Arc::clone(child));
        }

        while let Some(node) = stack.pop() {
            let current = node.lock().unwrap();
            if current.input.len() > *NODE_SIZE.get().unwrap() {
                let _unused =  Node::split_nodes(current);
            } else if !current.children.is_empty() {
                for child in &current.children {
                    stack.push(Arc::clone(child));
                }
            }

        }

        root

    }

    
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
    fn split_nodes(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;         // Mutable instance of self_node.

        self_instance.sort_main_nodes();

        let struct_one = Node::new(); // Holds keys smaller than middle key.
        let struct_two = Node::new(); // Holds keys larger than middle key.
        
        let items_size = self_instance.input.len();
        let breaking_point = (items_size + 1)/2;
        let temp_storage = self_instance.clone(); 
        let mut i = 0;
        self_instance.input.clear();
        for count in 1..temp_storage.input.len() + 1 {
            if count == breaking_point {
                self_instance.input.push(temp_storage.input[count-1].clone()); // Push the middle `Item` as sole parent.
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone()); // Push the `Items` with keys smaller than middle key onto struct_one.
                struct_one.lock().unwrap().input[count - 1].rank = temp_storage.rank + 1; // Set the key rank as parent node's rank + 1.
            } else if count > breaking_point {
                i = i + 1; // Variable "i" was used instead of count because `i` denotes the number of keys in struct_two.
                struct_two.lock().unwrap().input.push(temp_storage.input[count - 1].clone()); // Push the `Items` with keys larger than middle key onto struct_two.
                struct_two.lock().unwrap().input[i - 1].rank = temp_storage.rank + 1; // Set their key rank as parent node's rank + 1.
            } 
        }

        // Set struct_one/two's node rank as parent's node rank + 1.
        struct_one.lock().unwrap().rank = self_instance.rank + 1;
        struct_two.lock().unwrap().rank = self_instance.rank + 1;
        
        
        self_instance.children.push(struct_one);
        self_instance.children.push(struct_two);


        self_instance
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
    fn tree_integrity_check(mut self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut stack: Vec<Arc<Mutex<Node>>> = Vec::new();

        if self_node.input.len() + 1 != self_node.children.len() && !self_node.children.is_empty() {
            return Node::fix_child_count_mismatch(self_node);
        }

        for child in &self_node.children {
            stack.push(Arc::clone(child));
        }

        while let Some(node) = stack.pop() {
            let current = node.lock().unwrap();

            if current.input.len() + 1 != current.children.len() && !current.children.is_empty() {
                let _unused = Node::fix_child_count_mismatch(current);
            } else if !current.children.is_empty() {
                for child in &current.children {
                    stack.push(Arc::clone(child));
                }
            }
        }
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
    /// `Naïve O(N²)` algorithm *is* inefficient in comparison to `O(N Log N)` but is used as a placeholder to be replaced with (maybe) Sweep Line Algorithm. 
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
    fn fix_child_count_mismatch(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;
        let child_len = self_instance.children.len();
        
        for i in 0..child_len {
            // The children cannot be empty as only the nodes with children not empty can invoke the function (!self_instance.children.is_empty())
            let some_val = self_instance.children[i].lock().unwrap().clone();
            let some_val_input = &some_val.input;
            let keys_primary_required = vec![some_val_input[0].key, some_val_input.last().unwrap().key];
            for j in 0..child_len {
                let some_other_val = self_instance.children[j].lock().unwrap().clone();
                let some_other_val_input = &some_other_val.input;
                let keys_secondary_required = vec![some_other_val_input[0].key, some_other_val_input.last().unwrap().key];

                // Checks for overlapping node.
                if keys_primary_required[0] < keys_secondary_required[0] && keys_primary_required[1] > keys_secondary_required[1] {
                    let k = Arc::new(Mutex::new(some_val));
                    self_instance.children.push(k); // Pushes the overlapping node to the last index.
                    self_instance.children.remove(i); // Removes the unnecessary overlapping node.
                    break
                }
            }
        }


        let len = self_instance.children.len();
        if len >= 2 {
            // Extract keys + original index
            let mut key_nodes: Vec<_> = self_instance.children[len - 2..]
                .iter()
                .map(|node| {
                    let guard = node.lock().unwrap();
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
            let guard_parent_one = self_instance.children[self_instance.children.len() - 2].lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let guard_parent_two = self_instance.children[self_instance.children.len() - 1].lock().unwrap_or_else(|poisoned| poisoned.into_inner());

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
                        if require_child[0] > self_instance.input[j].key && require_child[1] < self_instance.input[j + 1].key {
                            placeholder = vec![self_instance.input[j].key, self_instance.input[j + 1].key]
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
            let k = self_instance.children[j].lock().unwrap().clone();
            if k.input[0].key > parent_one_child_boundary[0] && k.input[k.input.len() - 1].key < parent_one_child_boundary[1] {
                self_instance.children[self_instance.children.len()-2].lock().unwrap().children.push(Arc::new(Mutex::new(k)));
                self_instance.children.remove(j);
            } else if k.input[0].key > parent_two_child_boundary[0] && k.input[k.input.len() - 1].key < parent_two_child_boundary[1] {
                self_instance.children[self_instance.children.len()-1].lock().unwrap().children.push(Arc::new(Mutex::new(k)));
                self_instance.children.remove(j);
            } else {
                j += 1;
            }
        }
        self_instance
    }

    fn rank_correction(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;
        self_instance.sort_main_nodes();

        if !self_instance.children.is_empty() {
            for i in 0..self_instance.children.len() {
                self_instance.children[i].lock().unwrap().rank = self_instance.rank + 1;
                let len = self_instance.children[i].lock().unwrap().input.len();
                for j in 0..len {
                    self_instance.children[i].lock().unwrap().input[j].rank = self_instance.rank + 1;
                }

                let _unused = Node::rank_correction(self_instance.children[i].lock().unwrap());
            }
        }

        self_instance
    }
    fn add_child_key(&mut self, mut x: Items) -> () {
        if x.key < self.input[0].key {
            if !self.children.is_empty() {
                self.children[0].lock().unwrap().add_child_key(x);
            } else {
                x.rank = self.input[0].rank;
                self.input.push(x);
            }
        } else if x.key > self.input[self.input.len()-1].key {
            if !self.children.is_empty() {
                self.children[self.children.iter().count() - 1].lock().unwrap().add_child_key(x);
            } else {
                x.rank = self.input[0].rank;
                self.input.push(x);
            }
        } else {
            for i in 0..self.input.len() - 1 {
                if x.key > self.input[i].key && x.key < self.input[i+1].key {
                    if !self.children.is_empty() {
                        self.children[i+1].lock().unwrap().add_child_key(x.clone());
                    } else {
                        x.rank = self.input[0].rank;
                        self.input.push(x.clone());
                    }
                }
            }
        }
        self.sort_main_nodes();
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
    fn min_size_check(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut x = self_node;
        
        let mut indices_to_propagate = Vec::new();
        for (idx, child) in x.children.iter().enumerate() {
            let child_lock = child.lock().unwrap_or_else(|e| e.into_inner());
            if child_lock.input.len() < *NODE_SIZE.get().unwrap() / 2 && child_lock.rank > 1 {
                indices_to_propagate.push(idx);
            }
        }

        for &idx in indices_to_propagate.iter().rev() {
            let child_clone = x.children[idx].lock().unwrap_or_else(|e| e.into_inner()).clone();
            x = Node::propagate_up(x, child_clone);
        }

        for child in &x.children {
            let child_lock = child.lock().unwrap_or_else(|e| e.into_inner());
            if !child_lock.children.is_empty() {
                let _unused = Node::min_size_check(child_lock);
            }
        }
        
        x
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
    fn propagate_up(self_node: MutexGuard<Node>, mut child: Node) -> MutexGuard<Node> {
        let mut x = self_node;
        for i in 0..child.input.len() {
            child.input[i].rank = x.input[0].rank;
            x.input.push(child.input[i].clone());
        }
        for i in 0..child.children.len() {
            let meow = Arc::clone(&child.children[i]);
            x.children.push(meow);
        }

        let conditional_key = child.input[0].key;
        let mut to_be_removed = Vec::new();

        for i in 0..x.children.len() - 1 {
            let child_guard = x.children[i].lock().unwrap();
            if child_guard.input[0].key == conditional_key {
                to_be_removed.push(i);
            }
        }

        for i in to_be_removed.iter().rev() {
            x.children.remove(*i);
        }
        x.sort_main_nodes();
        x.sort_children_nodes();

        x
    }
    
    fn sort_children_nodes(&mut self) {
        self.children.sort_by(|a, b| {a.lock().unwrap().input[0].key.cmp(&b.lock().unwrap().input[0].key)});
    }
    fn sort_main_nodes(&mut self) {
        self.input.sort_by(|a, b| {a.key.cmp(&b.key)});
    }
    fn sort_everything(&mut self) {
        self.sort_main_nodes();
        self.sort_children_nodes();

        let children: Vec<Arc<Mutex<Node>>> = self.children.clone();

        for child in children {
            let mut child_guard = child.lock().unwrap();
            child_guard.sort_everything();
        }
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
    /// - Every key is sorted by ascending as [`Node::sort_everything`] is invoked before invoking [`Node::key_position`].
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
    fn key_position(node: Arc<Mutex<Node>>, key: u32) -> Option<Items> {
        let mut stack: Vec<Arc<Mutex<Node>>> = Vec::new();
        stack.push(node);

        while let Some(node) = stack.pop() {
            let current = node.lock().unwrap_or_else(|e| e.into_inner());

            for i in 0..current.input.len() {
                if current.input[i].key == key {
                    return Some(current.input[i].clone());
                }
            }

            if !current.children.is_empty() {
                if key < current.input[0].key {
                    stack.push(Arc::clone(&current.children[0]));
                } else if key > current.input[current.input.len()-1].key {
                    stack.push(Arc::clone(&current.children[current.children.len()-1]));
                } else {
                    for i in 0..current.input.len() - 1 {
                        if key > current.input[i].key && key < current.input[i+1].key {
                            stack.push(Arc::clone(&current.children[i+1]));
                        }
                    }
                }
            }
        }
        None
    }
    
    fn remove_key(self_node: &mut Arc<Mutex<Node>>, key: u32) {
        Node::remove_key_extension(self_node, key);
        let mut x = self_node.lock().unwrap();
        x = Node::removed_node_check(x);


    }
    fn remove_key_extension(self_node: &mut Arc<Mutex<Node>>, key: u32) {
        let mut x = self_node.lock().unwrap();
        let mut child_removed = false;
        for i in 0..x.input.len() {
            if x.input[i].key == key {
                child_removed = true;
                x.input.remove(i);
                break;
            }
        }

        if !child_removed {
            if key < x.input[0].key {
                return Node::remove_key_extension(&mut x.children[0], key);

            } else if key > x.input[x.input.len()-1].key {
                let k = x.children.len();
                return Node::remove_key_extension(&mut x.children[k-1], key);
            } else {
                for i in 0..x.input.len() - 1 {
                    if key > x.input[i].key && key < x.input[i+1].key {
                        return Node::remove_key_extension(&mut x.children[i+1], key);
                    }
                }
            }
        }
    }
    fn removed_node_check (self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut x = self_node;

        let mut indices_to_propagate = Vec::new();
        for (idx, child) in x.children.iter().enumerate() {
            let child_lock = child.lock().unwrap();
            if child_lock.input.len() < *NODE_SIZE.get().unwrap() / 2 && child_lock.rank > 1 {
                indices_to_propagate.push(idx);
            }
        }

        for &idx in indices_to_propagate.iter().rev() {
            x = Node::parent_key_down(x, idx);
        }

        for child in &x.children {
            let mut child_lock = child.lock().unwrap();
            if !child_lock.children.is_empty() {
                Node::removed_node_check(child_lock);
            }
        }

        x
    }
    fn parent_key_down(self_node: MutexGuard<Node>, idx: usize) -> MutexGuard<Node> {
        struct Value {
            difference: usize,
            index: usize,
        }

        let mut self_instance = self_node;
        let break_point = (self_instance.children.len() + 1) / 2;

        let mut child_with_keys = false;
        let mut index_vector = Vec::new();
        let mut index_vector_position = Vec::new();
        for i in 0..self_instance.children.len() {
            if self_instance.children[i].lock().unwrap().input.len() > *NODE_SIZE.get().unwrap() / 2 {
                child_with_keys = true;
                let mut k = 0;
                if idx > i {
                    k = idx - i;
                } else {
                    k = i - idx;
                }
                index_vector_position.push(Value{difference: k, index: i});
                index_vector.push(i);
            }
        }

        index_vector_position.sort_by(|a, b| a.difference.cmp(&b.difference));
        if child_with_keys {
            self_instance = Node::moving_keys(self_instance, idx, index_vector_position[0].index);
        } else if !child_with_keys {
            if idx + 1 < break_point {
                let k = self_instance.input[0].clone();
                let m = self_instance.children[1].lock().unwrap().input.clone();
                self_instance.input.remove(0);
                self_instance.children.remove(1);
                self_instance.children[0].lock().unwrap().input.push(k);
                for j in 0..m.len() {
                    self_instance.children[0].lock().unwrap().input.push(m[j].clone());

                }
            }

            if idx + 1 > break_point {
                let input_len = self_instance.input.len() - 1;
                let child_len = self_instance.children.len() - 1;
                let k = self_instance.input[input_len].clone();
                let m = self_instance.children[child_len].lock().unwrap().input.clone();
                self_instance.input.remove(input_len);
                self_instance.children.remove(child_len);
                self_instance.children[child_len - 1].lock().unwrap().input.push(k);
                for j in 0..m.len() {
                    self_instance.children[child_len - 1].lock().unwrap().input.push(m[j].clone());

                }
            }

            if idx + 1 == break_point {
                let k = self_instance.input[idx - 1].clone();
                let m = self_instance.children[idx - 1].lock().unwrap().input.clone();
                self_instance.input.remove(idx - 1);
                self_instance.children.remove(idx - 1);
                self_instance.children[idx - 1].lock().unwrap().input.push(k);
                for j in 0..m.len() {
                    self_instance.children[idx - 1].lock().unwrap().input.push(m[j].clone());

                }
            }
        }
        self_instance
    }
    fn moving_keys(self_node:MutexGuard<Node>, idx1: usize, idx2: usize) -> MutexGuard<Node> {
        let mut self_instance = self_node;

        if idx1 < idx2 {
            let m = self_instance.input[idx2-1].clone();
            let k = self_instance.children[idx2].lock().unwrap().input[0].clone();
            self_instance.input.remove(idx2 - 1);
            self_instance.children[idx2].lock().unwrap().input.remove(0);

            self_instance.input.push(k);
            self_instance.children[idx2 - 1].lock().unwrap().input.push(m);
        } else if idx1 > idx2 {
            let m = self_instance.input[idx2].clone();
            let len = self_instance.children[idx2].lock().unwrap().input.len();
            let k = self_instance.children[idx2].lock().unwrap().input[len - 1].clone();
            self_instance.input.remove(idx2);
            self_instance.children[idx2].lock().unwrap().input.remove(len - 1);

            self_instance.input.push(k);
            self_instance.children[idx2+1].lock().unwrap().input.push(m);

        }
        self_instance.sort_everything();
        if self_instance.children[idx1].lock().unwrap().input.len() < *NODE_SIZE.get().unwrap() / 2 {
            if idx1 < idx2 {
                self_instance = Node::moving_keys(self_instance, idx1, idx2-1);
            } else if idx1 > idx2 {
                self_instance = Node::moving_keys(self_instance, idx1, idx2+1);
            }
        }

        self_instance
    }

    fn all_keys_ordered(node: &Arc<Mutex<Node>>) -> Vec<Items> {
        let mut result = Vec::new();
        Self::collect_keys_inorder(node, &mut result);
        result
    }

    fn collect_keys_inorder(node: &Arc<Mutex<Node>>, result: &mut Vec<Items>) {
        let node_instance = node.lock().unwrap();

        if node_instance.children.is_empty() {
            for i in 0..node_instance.input.len() {
                result.push(node_instance.input[i].clone());
            }
        } else {
            for i in 0..node_instance.input.len() {
                Node::collect_keys_inorder(&node_instance.children[i], result);
                result.push(node_instance.input[i].clone());
            }
            Node::collect_keys_inorder(&node_instance.children[node_instance.input.len()], result);
        }
    }
    
    fn serialize(node: &Arc<Mutex<Node>>) -> io::Result<()>  {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("example.txt")?;
        
        writeln!(file, "[0]").expect("TODO: panic message");
        Node::serialization(node, &mut file);
        Ok(())
    }
    
    fn serialization(node: &Arc<Mutex<Node>>, file: &mut File) {
        let node_instance = node.lock().unwrap();

        let l = node_instance.input.len();

        writeln!(file, "[{:X}]", node_instance.rank).expect("Error writing to file.");
        writeln!(file, "[{:X}]", l).expect("panic message");
        for i in 0..l {
            write!(file, "[{}]", node_instance.input[i].key).expect("panic message");
            let value_len = node_instance.input[i].value.len();
            writeln!(file, "[{}]", value_len).expect("panic message");
            let x : Vec<char> = node_instance.input[i].value.chars().collect();
            write!(file, "{:?}", x).expect("panic message");
            writeln!(file,"").expect("panic message");
        }
        writeln!(file,"[{:X}]", node_instance.children.len()).expect("panic message");

        if !node_instance.children.is_empty() {
            for i in 0..node_instance.input.len() {
                Node::serialization(&node_instance.children[i], file);
            }
            Node::serialization(&node_instance.children[node_instance.input.len()], file);
        }
    }

    fn deserialize() -> io::Result<()> {
        let file = File::open("example.txt")?;
        let read = BufReader::new(file);

        let single_bracket = Regex::new(r"^\[[^\]]+\]$").unwrap();
        let double_bracket = Regex::new(r"^\[[^\]]+\]\[[^\]]+\]$").unwrap();
        let array_pattern = Regex::new(r"^\[('[^']*'(,\s*'[^']*')*)\]$").unwrap();

        let mut vec: Vec<U32OrString> = Vec::new();
        
        for contents in read.lines() {
            let x = contents?;
            let k = x.as_str();
            
            if array_pattern.is_match(k) {
                let result: String = k
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(", ")
                    .map(|char_str| char_str.trim_matches('\'').chars().next().unwrap())
                    .collect();

                vec.push(U32OrString::Str(result));
                // println!("{}", result);
            }

            else if single_bracket.is_match(k) || double_bracket.is_match(k) {
                let chars: Vec<char> = k.chars().collect();
                let mut numbers = Vec::new();
                let mut current_num = String::new();
                let mut inside_brackets = false;

                for &ch in &chars {
                    match ch {
                        '[' => inside_brackets = true,
                        ']' => {
                            if inside_brackets && !current_num.is_empty() {
                                numbers.push(current_num.parse::<u32>().expect("Error parsing number"));
                                current_num.clear();
                            }
                            inside_brackets = false;
                        }
                        digit if digit.is_ascii_digit() && inside_brackets => {
                            current_num.push(digit);
                        }
                        _ => {}
                    }
                }

                if numbers.len() == 2 {
                    vec.push(U32OrString::Num(numbers[0]));
                    vec.push(U32OrString::Num(numbers[1]));
                } else if numbers.len() == 1 {
                    vec.push(U32OrString::Num(numbers[0]));
                }
            }
        }

        let vector_len = vec.len();
        let mut count = 0;
        let mut internal_count = 0;
        let mut vec_items: Vec<Items> = Vec::new();
        let mut node_vec: Vec<DeserializedNode> = Vec::new();
        let mut no_of_keys_helper_counter = 0;
        let mut first_time_hit_item_push = true;
        let mut rank_for_keys = 0;
        let mut push_count = 0;
        for _i in 0..vector_len {
            let mut no_of_keys =0;
            count = count + 1;


            if count > 3 {
                if let U32OrString::Num(value) = &vec[no_of_keys_helper_counter + 2] {
                    no_of_keys = *value;
                }

                internal_count = internal_count + 1;
                if (no_of_keys * 3 + 4) as usize == count && count > (no_of_keys * 3) as usize{
                    let mut k = 0;
                    if let U32OrString::Num(value) = &vec[count -1] {
                        k = *value;
                    }
                }
            }
            
            if internal_count == (no_of_keys * 3 + 1) as usize {
                let mut probable_child_count = 0;
                if let U32OrString::Num(value) = &vec[count -1] {
                    probable_child_count = *value;
                }
                if no_of_keys == push_count && !vec_items.is_empty() {
                    node_vec.push(DeserializedNode { items:vec_items.clone(), child_count: probable_child_count });
                    push_count = 0;
                }

            }
            
            if internal_count >= (no_of_keys * 3 + 3) as usize {
                vec_items.clear();
                no_of_keys_helper_counter = no_of_keys_helper_counter + (no_of_keys * 3 + 3) as usize;
                internal_count = 0;
                first_time_hit_item_push = true;
            }
            if internal_count % 3 == 0 && internal_count >= 3 {
                let mut k = 0;
                let mut l = String::new();

                if first_time_hit_item_push {
                    if let U32OrString::Num(value) = &vec[count - 5] {
                        rank_for_keys = *value;
                    }
                    first_time_hit_item_push = false;
                }
                
                if let U32OrString::Num(value) = &vec[count - 3] {
                    k = *value;

                }

                if let U32OrString::Str(value) = &vec[count - 1] {
                    l = value.clone();
                }

                vec_items.push(Items{key:k, value: l, rank: rank_for_keys });
                push_count += 1;
            }
            
        }

        let required_node = node_vec[0].clone();
        let x = Node::deserialized_with_relation(required_node, &mut node_vec);

        let mut k = Node::deserialized_data_to_nodes(x);
        k = Node::deserialized_duplicate_data_check(k);

        println!("###############################################");
        println!("{:?}", k.print_tree());
        println!("###############################################");


        Ok(())
    }

    fn deserialized_with_relation(required_node: DeserializedNode, node_vec:&mut  Vec<DeserializedNode>) -> UltraDeserialized {
        let mut x = UltraDeserialized {parent: required_node.clone(), children: Vec::new()};
        if required_node.child_count > 0 {
            let mut i = 0;
            while i < node_vec.len() && required_node.child_count > x.children.len() as u32 {
                if required_node.items[0].rank + 1 == node_vec[i].items[0].rank {
                    x.children.push(UltraDeserialized {parent: node_vec[i].clone(), children: Vec::new()});
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
                    z = Node::deserialized_with_relation(x.children[i].parent.clone(), node_vec);
                    x.children.push(z);
                }
           }
        }
        x
    }

    fn deserialized_duplicate_data_check(self_node: Node) -> Node {
        let mut self_instance = self_node;
        let mut dup_child_len = self_instance.children.len()/2;
        
/*        for i in 0..child_len {
            let child_2_len = self_instance.children[i].lock().unwrap().children.len();
            println!(" child_2_len: {} {:?}", child_2_len, self_instance.print_tree());
            
            let inp_len = self_instance.children[i].lock().unwrap().input.len();
            if inp_len + 1 != child_2_len {
                self_instance.children.remove(0);
            }
        }*/

        let mut i = 0;
        while i < self_instance.children.len() {
            let first_child = self_instance.children[i].lock().unwrap().input.clone();
            let mut found_duplicate = false;

            for j in (i + 1)..self_instance.children.len() {
                let second_child = self_instance.children[j].lock().unwrap().input.clone();

                if first_child[0] == second_child[0] {
                    self_instance.children.remove(i);
                    found_duplicate = true;
                    break;
                }
            }

            if !found_duplicate {
                i += 1;
            }
        }

        for i in 0..dup_child_len {
            let child_child_len = self_instance.children[i].lock().unwrap().children.len();
            let mut x = self_instance.children[i].lock().unwrap().clone();
            if child_child_len > 0 {
                self_instance.children[i] = Arc::new(Mutex::new(Node::deserialized_duplicate_data_check(x)));
            }
        }

        self_instance

    }

    fn deserialized_data_to_nodes(deserialized_data: UltraDeserialized) -> Node {
        let mut new_node = Node{input: Vec::new(), rank: 0, children: Vec::new()};

        new_node.input = deserialized_data.parent.items.clone();
        new_node.rank = deserialized_data.parent.items[0].rank;

        if !deserialized_data.children.is_empty() {
            let child_len = deserialized_data.children.len();
            let mut child_vec = Vec::new();
            for j in 0..child_len {
                let k = Arc::new(Mutex::new(Node::deserialized_data_to_nodes(deserialized_data.children[j].clone())));
                child_vec.push(k);
            }
            new_node.children = child_vec;
        }

        new_node
    }

}

fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let mut new_node = Node::new();
    let mut c = 0;
/*    for i in 0..50 {
        let sec = rand::thread_rng().gen_range(1, 1000);
        Node::insert(&mut new_node, sec, String::from("Woof"));
        c = c + 1;
        println!("{} - {}", c, sec);
    }*/
    
    

    Node::insert(&mut new_node, 1, String::from("Woof"));
    Node::insert(&mut new_node, 2, String::from("Woof"));
    Node::insert(&mut new_node, 3, String::from("Woof"));
    Node::insert(&mut new_node, 4, String::from("Woof"));
    Node::insert(&mut new_node, 5, String::from("Woof"));
    Node::insert(&mut new_node, 6, String::from("Woof"));
    Node::insert(&mut new_node, 7, String::from("Woof"));
    Node::insert(&mut new_node, 8, String::from("Woof"));
    Node::insert(&mut new_node, 9, String::from("Woof"));
    Node::insert(&mut new_node, 10, String::from("Woof"));
    Node::insert(&mut new_node, 11, String::from("Woof"));
    Node::insert(&mut new_node, 12, String::from("Woof"));
    Node::insert(&mut new_node, 13, String::from("Woof"));
    Node::insert(&mut new_node, 14, String::from("Woof"));
    Node::insert(&mut new_node, 15, String::from("Woof"));

    println!("{:?}", new_node.lock().unwrap().print_tree());

    println!("Key to be discovered?");
    let required_key = read_num();


        match Node::key_position(new_node.clone(), required_key) {
            Some(x) => {
                println!("Key found");
                println!("{:?}", x);
            }
            None => println!("Key not found"),
        }

/*    for i in 0..100 {
        println!("Keys to be deleted?");
        let required_key = read_num();
        Node::remove_key(&mut new_node, required_key);
        println!("{:?}", new_node.lock().unwrap().print_tree());
    }
    
    let k = Node::all_keys_ordered(&mut new_node);
    for i in 0..k.len() {
        println!("{} - {}", k[i].key, k[i].value);
    }
    
    Node::serialize(&new_node).expect("panic message");
    Node::deserialize().expect("panic message");
*/}

fn read_num() -> u32 {
    let mut inp = String::new();
    io::stdin().read_line(&mut inp).expect("Failed to read line");
    let n = inp.trim().parse().expect("Not a number");
    n
}
fn read_string() -> String {
    let mut inp = String::new();
    io::stdin().read_line(&mut inp).expect("Failed to read line");
    inp.trim().to_string()
}

impl Node {
    /// Pretty print the entire tree starting from this node
    pub fn print_tree(&self) {
        self.print_tree_recursive("", true, 0);
    }

    
    /// Recursive helper for tree printing
    fn print_tree_recursive(&self, prefix: &str, is_last: bool, depth: usize) {
        // Print current node
        let connector = if depth == 0 {
            "Root"
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        println!("{}{}Node(rank: {}) [{}]",
                 prefix,
                 connector,
                 self.rank,
                 self.format_items());

        // Prepare prefix for children
        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        // Print children
        for (i, child_arc) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;

            match child_arc.lock() {
                Ok(child) => {
                    child.print_tree_recursive(&child_prefix, is_last_child, depth + 1);
                }
                Err(_) => {
                    println!("{}{}[POISONED MUTEX]",
                             child_prefix,
                             if is_last_child { "└── " } else { "├── " });
                }
            }
        }
    }

    /// Format the items in a readable way
    fn format_items(&self) -> String {
        if self.input.is_empty() {
            return "empty".to_string();
        }

        let items: Vec<String> = self.input
            .iter()
            .map(|item| format!("{}:{} ({})", item.key, item.value, item.rank))
            .collect();

        items.join(", ")
    }

    /// Alternative compact horizontal view
    pub fn print_compact(&self) {
        println!("B-Tree Structure:");
        println!("{}", "=".repeat(50));
        self.print_compact_recursive(0);
    }

    fn print_compact_recursive(&self, level: usize) {
        let indent = "  ".repeat(level);
        println!("{}Level {}: [{}] (rank: {})",
                 indent,
                 level,
                 self.format_items(),
                 self.rank);

        for (i, child_arc) in self.children.iter().enumerate() {
            match child_arc.lock() {
                Ok(child) => {
                    if i == 0 && !self.children.is_empty() {
                        println!("{}Children:", "  ".repeat(level + 1));
                    }
                    child.print_compact_recursive(level + 1);
                }
                Err(_) => {
                    println!("{}[POISONED MUTEX]", "  ".repeat(level + 1));
                }
            }
        }
    }

    /// Tree statistics
    pub fn print_stats(&self) {
        let stats = self.calculate_stats();
        println!("Tree Statistics:");
        println!("├── Total nodes: {}", stats.total_nodes);
        println!("├── Tree height: {}", stats.height);
        println!("├── Total keys: {}", stats.total_keys);
        println!("├── Leaf nodes: {}", stats.leaf_nodes);
        println!("└── Internal nodes: {}", stats.internal_nodes);
    }

    fn calculate_stats(&self) -> TreeStats {
        let mut stats = TreeStats::default();
        self.calculate_stats_recursive(&mut stats, 0);
        stats
    }

    fn calculate_stats_recursive(&self, stats: &mut TreeStats, depth: usize) {
        stats.total_nodes += 1;
        stats.total_keys += self.input.len();
        stats.height = stats.height.max(depth + 1);

        if self.children.is_empty() {
            stats.leaf_nodes += 1;
        } else {
            stats.internal_nodes += 1;
            for child_arc in &self.children {
                if let Ok(child) = child_arc.lock() {
                    child.calculate_stats_recursive(stats, depth + 1);
                }
            }
        }
    }
}
#[derive(Default)]
struct TreeStats {
    total_nodes: usize,
    height: usize,
    total_keys: usize,
    leaf_nodes: usize,
    internal_nodes: usize,
}
