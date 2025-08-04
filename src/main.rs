use std::cell::RefCell;
use crate::rand::Rng;
use std::io::{BufRead, BufReader, Write};
extern crate rand;

use std::{env, fs, io};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use once_cell::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::mem::zeroed;
use std::path::Path;
use std::thread;
use std::time::Duration;
use clap::{Parser, Subcommand};


static NODE_SIZE: OnceCell<usize> = OnceCell::new();
static COUNTER: AtomicUsize = AtomicUsize::new(100);
static CHECKPOINT_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
struct Version {
    value: String,
    xmin: u32,
    xmax: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
struct Items {
    key: u32,
    rank: u32,
    version: Vec<Version>
}

#[derive(Debug, Clone)]
struct Node {
    input: Vec<Items>,
    rank: u32,
    children: Vec<Arc<RwLock<Node>>>,
}

#[derive(Debug)]
enum I32OrString {
    Num(i32),
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
    fn new() -> Arc<RwLock<Node>> {
        let instance = Arc::new(RwLock::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
        }));
        instance
    }

    // Insert the K-V into the empty node.
    // Todo: Understand why i had to call every function 3 times for correct functioning.
    fn insert(mut self_node: Arc<RwLock<Node>>, k: u32, v: String, txn: u32) -> io::Result<()> {
        {
            let ver = Version {value: v.clone(), xmin: txn, xmax: None};
            if Node::find_and_update_key_version(Arc::clone(&self_node), k, v.clone(), txn) == None {
                println!("New version");
                let version = vec!(ver.clone());
                Node::add_new_keys(Arc::clone(&self_node), Items { key: k, rank: 1, version });
            }
        }


        self_node = Node::overflow_check(self_node);
        self_node = Node::min_size_check(self_node);
        self_node = Node::sort_main_nodes(self_node);
        self_node = Node::sort_children_nodes(self_node);

        self_node = Node::tree_integrity_check(self_node);


        self_node = Node::min_size_check(self_node);

        self_node = Node::overflow_check(self_node);

        self_node = Node::min_size_check(self_node);
        self_node = Node::sort_main_nodes(self_node);
        self_node = Node::tree_integrity_check(self_node);
        self_node = Node::rank_correction(self_node);
        self_node = Node::sort_everything(self_node);
        
        self_node = Node::overflow_check(self_node);
        self_node = Node::min_size_check(self_node);
        self_node = Node::tree_integrity_check(self_node);
        self_node = Node::rank_correction(self_node);
        self_node = Node::sort_everything(self_node);

        Ok(())
    }

    /// # THIS IS A TEMPORARY HACK SOLUTION. IT'LL STAY THERE TILL I ADD AN ACTUAL THREAD SAFE FUNCTION.
    /// ## DO NOT TAKE THIS SERIOUSLY.
    /// ### :(
    fn find_and_update_key_version(node: Arc<RwLock<Node>>, key: u32, v: String, txn: u32) -> Option<()> {
        let mut write_guard = {
            let w1 = node.write();
            w1.unwrap_or_else(|poisoned| poisoned.into_inner())
        };
        for i in 0..write_guard.input.len() {
            if write_guard.input[i].key == key {
                let ver_count = write_guard.input[i].version.len();
                let ver = Version {value: v.clone(), xmin: txn, xmax: None};
                write_guard.input[i].version[ver_count - 1].xmax = Option::from(txn);
                write_guard.input[i].version.push(ver);
                return Some(());
            }
        }
        drop(write_guard);
        let read_guard = node.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !read_guard.children.is_empty() {
            println!("C");
            if key < read_guard.input[0].key {
                let guard = Arc::clone(&read_guard.children[0]);
                drop(read_guard);
                return Node::find_and_update_key_version(guard, key, v, txn);
            } else if key > read_guard.input[read_guard.input.len()-1].key {
                let guard = Arc::clone(&read_guard.children.last().unwrap());
                drop(read_guard);
                return Node::find_and_update_key_version(guard, key, v, txn);
            } else {
                for i in 0..read_guard.input.len() - 1 {
                    if key > read_guard.input[i].key && key < read_guard.input[i+1].key {
                        let guard = Arc::clone(&read_guard.children[i+1]);
                        drop(read_guard);
                        return Node::find_and_update_key_version(guard, key, v, txn);
                    }
                }
            }
        }
        None
    }
    
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
    fn overflow_check(root: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
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
                let _unused =  Node::split_nodes(current_clone);
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
    fn split_nodes(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        self_node = Node::sort_main_nodes(self_node);
        let mut self_instance = self_node.write().unwrap_or_else(|e| e.into_inner());      // Mutable instance of self_node.
        
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
                struct_one.write().unwrap().input.push(temp_storage.input[count-1].clone()); // Push the `Items` with keys smaller than middle key onto struct_one.
                struct_one.write().unwrap().input[count - 1].rank = temp_storage.rank + 1; // Set the key rank as parent node's rank + 1.
            } else if count > breaking_point {
                i = i + 1; // Variable "i" was used instead of count because `i` denotes the number of keys in struct_two.
                struct_two.write().unwrap().input.push(temp_storage.input[count - 1].clone()); // Push the `Items` with keys larger than middle key onto struct_two.
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
    fn tree_integrity_check(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut stack = Vec::new();

        let self_instance = self_node.read().unwrap_or_else(|poisoned| poisoned.into_inner());

        if self_instance.input.len() + 1 != self_instance.children.len() && !self_instance.children.is_empty() {
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
            if current_instance.input.len() + 1 != current_instance.children.len() && !current_instance.children.is_empty() {
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
    fn fix_child_count_mismatch(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_instance = self_node.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        let child_len = self_instance.children.len();

        for i in 0..child_len {
            // The children cannot be empty as only the nodes with children not empty can invoke the function (!self_instance.children.is_empty())
            let some_val = self_instance.children[i].read().unwrap().clone();
            let some_val_input = &some_val.input;
            let keys_primary_required = vec![some_val_input[0].key, some_val_input.last().unwrap().key];
            for j in 0..child_len {
                let some_other_val = self_instance.children[j].read().unwrap().clone();
                let some_other_val_input = &some_other_val.input;
                let keys_secondary_required = vec![some_other_val_input[0].key, some_other_val_input.last().unwrap().key];

                // Checks for overlapping node.
                if keys_primary_required[0] < keys_secondary_required[0] && keys_primary_required[1] > keys_secondary_required[1] {
                    let k = Arc::new(RwLock::new(some_val));
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
            let guard_parent_one = self_instance.children[self_instance.children.len() - 2].read().unwrap_or_else(|p| p.into_inner());
            let guard_parent_two = self_instance.children[self_instance.children.len() - 1].read().unwrap_or_else(|p| p.into_inner());

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
            let k = self_instance.children[j].read().unwrap().clone();
            if k.input[0].key > parent_one_child_boundary[0] && k.input[k.input.len() - 1].key < parent_one_child_boundary[1] {
                self_instance.children[self_instance.children.len()-2].write().unwrap().children.push(Arc::new(RwLock::new(k)));
                self_instance.children.remove(j);
            } else if k.input[0].key > parent_two_child_boundary[0] && k.input[k.input.len() - 1].key < parent_two_child_boundary[1] {
                self_instance.children[self_instance.children.len()-1].write().unwrap().children.push(Arc::new(RwLock::new(k)));
                self_instance.children.remove(j);
            } else {
                j += 1;
            }
        }
        drop(self_instance);
        self_node
    }

    fn rank_correction(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
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

    fn add_new_keys(self_node : Arc<RwLock<Node>>, mut x: Items){
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
            } else if x.key > self_instance.input[self_instance.input.len()-1].key {
                if !self_instance.children.is_empty() {
                    Node::add_new_keys(Arc::clone(&self_instance.children[self_instance.children.len()-1]), x);
                } else {
                    x.rank = self_instance.input[0].rank;
                    self_instance.input.push(x);
                }
            } else {
                for i in 0..self_instance.input.len() - 1 {
                    if x.key > self_instance.input[i].key && x.key < self_instance.input[i+1].key {
                        if !self_instance.children.is_empty() {
                            Node::add_new_keys(Arc::clone(&self_instance.children[i+1]), x.clone());
                        } else {
                            x.rank = self_instance.input[0].rank;
                            self_instance.input.push(x.clone());
                        }
                    }
                }
            }
        }
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
    fn min_size_check(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
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
    fn propagate_up(mut self_node: Arc<RwLock<Node>>,child: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let self_read = self_node.read().unwrap();
        let mut child_write = child.write().unwrap();

        for child_input in &mut child_write.input {
            child_input.rank = self_read.rank;
        }
        drop(self_read);
        drop(child_write);

        let mut self_write = self_node.write().unwrap();
        let mut child_read = child.read().unwrap();

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

    fn sort_children_nodes(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_write = self_node.write().unwrap();
        self_write.children.sort_by(|a, b| {a.read().unwrap().input[0].key.cmp(&b.read().unwrap().input[0].key)});
        drop(self_write);
        self_node
    }
    fn sort_main_nodes(self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
        let mut self_write = self_node.write().unwrap();
        self_write.input.sort_by(|a, b| {a.key.cmp(&b.key)});
        drop(self_write);
        self_node
    }
    fn sort_everything(mut self_node: Arc<RwLock<Node>>) -> Arc<RwLock<Node>> {
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
    fn key_position(node: Arc<RwLock<Node>>, key: u32) -> Option<Vec<Version>> {
        let mut stack = Vec::new();
        stack.push(node);

        while let Some(self_node) = stack.pop() {
            let current = self_node.read().unwrap_or_else(|e| e.into_inner());

            for i in 0..current.input.len() {
                if current.input[i].key == key {
                    return Some(current.input[i].version.clone());
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
    fn remove_key(self_node: Arc<RwLock<Node>>, key: u32) {
        let cloned = Arc::clone(&self_node);
        Node::remove_key_extension(self_node, key);
        Node::removed_node_check(cloned);
    }
    fn remove_key_extension(self_node: Arc<RwLock<Node>>, key: u32) {
        let mut child_removed = false;
        let mut x = self_node.write().unwrap();
        if let Some(p) = x.input.iter().position(|item| item.key == key) {
            child_removed = true;
            println!("Deleted the key at rank {:?}", x.rank);
            x.input.remove(p);
        }

        drop(x);
        let x = self_node.read().unwrap();
        
        if !child_removed {
            if key < x.input[0].key {
                let cloned = Arc::clone(&x.children[0]);
                return Node::remove_key_extension(cloned, key);

            } else if key > x.input[x.input.len()-1].key {
                let cloned = Arc::clone(&x.children.last().unwrap());
                return Node::remove_key_extension(cloned, key);
            } else {
                for i in 0..x.input.len() - 1 {
                    if key > x.input[i].key && key < x.input[i+1].key {
                        let cloned = Arc::clone(&x.children[i+1]);
                        return Node::remove_key_extension(cloned, key);
                    }
                }
            }
        }
    }
    fn removed_node_check (mut self_node: Arc<RwLock<Node>>) {
        let read_guard = self_node.read().unwrap();

        let mut indices_to_propagate = Vec::new();
        for (idx, child) in read_guard.children.iter().enumerate() {
            let child_lock = child.read().unwrap();
            if child_lock.input.len() < *NODE_SIZE.get().unwrap() / 2 && child_lock.rank > 1 {
                indices_to_propagate.push(idx);
            }
        }
        
        drop(read_guard);

        for &idx in indices_to_propagate.iter().rev() {
            let meow = Arc::clone(&self_node);
            self_node = Node::parent_key_down(meow, idx);
        }
        
        let read_guard = self_node.read().unwrap();
        for child in &read_guard.children {
            let mut child_lock = child.read().unwrap();
            if !child_lock.children.is_empty() {
                drop(child_lock);
                Node::removed_node_check(child.clone());
            }
        }
        
    }
    fn parent_key_down(self_node: Arc<RwLock<Node>>, idx: usize) -> Arc<RwLock<Node>> {
        struct Value {
            difference: usize,
            index: usize,
        }

        let mut self_instance = self_node.read().unwrap();
        let break_point = (self_instance.children.len() + 1) / 2;

        let mut child_with_keys = false;
        let mut index_vector = Vec::new();
        let mut index_vector_position = Vec::new();
        for i in 0..self_instance.children.len() {
            let input_len = {
                let child_guard = self_instance.children[i].read().unwrap();
                child_guard.input.len()
            };
            
            if input_len > *NODE_SIZE.get().unwrap() / 2 {
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
        
        drop(self_instance);
        
        index_vector_position.sort_by(|a, b| a.difference.cmp(&b.difference));
        if child_with_keys {
            let k = Arc::clone(&self_node);
            Node::moving_keys(k, idx, index_vector_position[0].index);
        } else if !child_with_keys {
            // TODO: Modify the given statement by removing 0 and 1.
            if idx + 1 < break_point {
                let self_guard = &mut self_node.write().unwrap();
                
                let k = self_guard.input[0].clone();

                let m = {
                    let b = self_guard.children[1].read().unwrap();
                    let c = b.input.clone();
                    c
                };
                
                self_guard.input.remove(0);
                self_guard.children.remove(1);
                
                // Do we need &mut here?
                let child_guard = &mut self_guard.children[0].write().unwrap();
                child_guard.input.push(k);
                child_guard.input.extend(m);
            }

            if idx + 1 > break_point {
                let self_guard = &mut self_node.write().unwrap();
                
                let input_len = self_guard.input.len() - 1;
                let child_len = self_guard.children.len() - 1;
                let k = self_guard.input[input_len].clone();
                
                let m = {
                    let b = self_guard.children[child_len].read().unwrap();
                    b.input.clone()
                };
                self_guard.input.remove(input_len);
                self_guard.children.remove(child_len);
                
                let child_guard = &mut self_guard.children[child_len - 1].write().unwrap();
                
                child_guard.input.push(k);
                child_guard.input.extend(m);
            }

            if idx + 1 == break_point {
                let self_guard = &mut self_node.write().unwrap();
                
                let k = self_guard.input[idx - 1].clone();
                let m = {
                    let b = self_guard.children[idx - 1].read().unwrap();
                    b.input.clone()
                };

                self_guard.input.remove(idx - 1);
                self_guard.children.remove(idx - 1);
                
                let child_guard = &mut self_guard.children[idx - 1].write().unwrap();
                child_guard.input.push(k);
                child_guard.input.extend(m);
            }
        }
        self_node
    }
    fn moving_keys(mut self_node: Arc<RwLock<Node>>, idx1: usize, idx2: usize) {
        if idx1 < idx2 {
            let self_guard = &mut self_node.write().unwrap();
            let m = self_guard.input[idx2 - 1].clone();
            let k = {
                let child_guard = self_guard.children[idx2].read().unwrap();
                child_guard.input[0].clone()
            };
            self_guard.input.remove(idx2 - 1);         
            self_guard.input.push(k);

            {
                let child_guard = &mut self_guard.children[idx2].write().unwrap();
                child_guard.input.remove(0);
            }
            {
                let child_guard = &mut self_guard.children[idx2 - 1].write().unwrap();
                child_guard.input.push(m);
            }
        } else if idx1 > idx2 {
            let self_guard = &mut self_node.write().unwrap();
            
            let m = self_guard.input[idx2].clone();
            
            let (k,len) = {
                let child_guard = self_guard.children[idx2].read().unwrap();
                let a = child_guard.input.len();
                let b = child_guard.input[a - 1].clone();
                (b,a)
            };
            self_guard.input.remove(idx2);
            self_guard.input.push(k);
            {
                let child_guard = &mut self_guard.children[idx2].write().unwrap();
                child_guard.input.remove(len - 1);
            }
            {
                let child_guard = &mut self_guard.children[idx2 + 1].write().unwrap();
                child_guard.input.push(m);
            }
        }
        
        self_node = Node::sort_everything(self_node);
        let length = {
            let self_guard = self_node.read().unwrap();
            let child_guard = self_guard.children[idx1].read().unwrap();
            child_guard.input.len()
        };
        if length < *NODE_SIZE.get().unwrap() / 2 {
            if idx1 < idx2 {
                let k = Arc::clone(&self_node);
                Node::moving_keys(k, idx1, idx2-1);
            } else if idx1 > idx2 {
                let k = Arc::clone(&self_node);
                Node::moving_keys(k, idx1, idx2+1);
            }
        }
    }

    fn all_keys_ordered(node: Arc<RwLock<Node>>) -> Vec<Items> {
        let mut result = Vec::new();
        Self::collect_keys_inorder(node, &mut result);
        result
    }

    fn collect_keys_inorder(node: Arc<RwLock<Node>>, result: &mut Vec<Items>) {
        let node_instance = node.read().unwrap();

        if node_instance.children.is_empty() {
            for i in 0..node_instance.input.len() {
                result.push(node_instance.input[i].clone());
            }
        } else {
            for i in 0..node_instance.input.len() {
                Node::collect_keys_inorder(Arc::clone(&node_instance.children[i]), result);
                result.push(node_instance.input[i].clone());
            }
            Node::collect_keys_inorder(Arc::clone(&node_instance.children[node_instance.input.len()]), result);
        }
    }

    fn serialize(node: Arc<RwLock<Node>>) -> io::Result<()>  {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("/home/_meringue/RustroverProjects/ASMT-V1/example.txt")?;

        writeln!(file, "[0]").expect("TODO: panic message");
        Node::serialization(node, &mut file);
        Ok(())
    }

    fn serialization(node: Arc<RwLock<Node>>, file: &mut File) {
        let node_instance = node.read().unwrap();
        let l = node_instance.input.len();
        writeln!(file, "[{:X}]", node_instance.rank).expect("Error writing to file.");
        writeln!(file, "[{:X}]", l).expect("panic message");
        for i in 0..l {
            write!(file, "[{}]", node_instance.input[i].key).expect("panic message");
            let version_len = node_instance.input[i].version.len();
            writeln!(file, "[{}]", version_len).expect("panic message");
            for ver in &node_instance.input[i].version {
                write!(file, "[{}]", ver.xmin).expect("panic message");
                match ver.xmax {
                    Some(xm) => {
                        write!(file, "[{}]", xm).expect("panic message");
                    }

                    None => {
                        write!(file, "[-]").expect("panic message");
                    }
                }
                let value_len = ver.value.len();
                writeln!(file, "[{}]", value_len).expect("panic message");
                let x : Vec<char> = ver.value.chars().collect();
                write!(file, "{:?}", x).expect("panic message");
                writeln!(file,"").expect("panic message");

            }
        }
        writeln!(file,"[{:X}]", node_instance.children.len()).expect("panic message");
        if !node_instance.children.is_empty() {
            for i in 0..node_instance.input.len() + 1 {
                let z = Arc::clone(&node_instance.children[i]);
                Node::serialization(z, file);
            }
        }
    }

    fn deserialize(serialized_file_path: &str) -> io::Result<(Arc<RwLock<Node>>)> {
        let file = File::open(serialized_file_path.clone())?;
        let metadata = fs::metadata(serialized_file_path)?;
        if metadata.len() == 0 {
            return Ok(Node::new());
        }

        let read = BufReader::new(file);


        let single_bracket = Regex::new(r"^\[[^\]]+\]$").unwrap();
        let double_bracket = Regex::new(r"^\[[^\]]+\]\[[^\]]+\]$").unwrap();
        let triple_bracket = Regex::new(r"^\[[^\]]+\]\[[^\]]+\]\[[^\]]+\]$").unwrap();
        let array_pattern = Regex::new(r"^\[('[^']*'(,\s*'[^']*')*)\]$").unwrap();

        let mut vec: Vec<I32OrString> = Vec::new();

        for contents in read.lines() {
            let x = contents?;
            let k = x.as_str();

            if array_pattern.is_match(k) {
                let result: String = k
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(", ")
                    .map(|char_str| char_str.trim_matches('\'').chars().next().unwrap())
                    .collect();

                vec.push(I32OrString::Str(result));
            }

            else if single_bracket.is_match(k) || double_bracket.is_match(k) || triple_bracket.is_match(k) {
                let chars: Vec<char> = k.chars().collect();
                let mut numbers = Vec::new();
                let mut current_num = String::new();
                let mut inside_brackets = false;

                for &ch in &chars {
                    match ch {
                        '[' => inside_brackets = true,
                        ']' => {
                            if inside_brackets && !current_num.is_empty() {
                                if current_num == "-" {
                                    println!("A");
                                    numbers.push(-1);
                                } else {
                                    numbers.push(current_num.parse::<i32>().expect("Error parsing number"));
                                }
                                current_num.clear();
                            }
                            inside_brackets = false;
                        }
                        digit if digit.is_ascii_digit() && inside_brackets || inside_brackets && digit == '-' => {
                            current_num.push(digit);
                        }
                        _ => {}
                    }
                }

                if numbers.len() == 2 {
                    vec.push(I32OrString::Num(numbers[0]));
                    vec.push(I32OrString::Num(numbers[1]));
                } else if numbers.len() == 1 {
                    vec.push(I32OrString::Num(numbers[0]));
                } else if numbers.len() == 3 {
                    vec.push(I32OrString::Num(numbers[0]));
                    vec.push(I32OrString::Num(numbers[1]));
                    vec.push(I32OrString::Num(numbers[2]));
                }
            }
        }

/*        for item in vec.iter() {
            println!("{:?}", item);
        }*/
        let vector_len = vec.len();
        let mut count = 0;
        let mut no_of_keys_in_node = 0;
        let mut keys = 0;
        let mut version_of_single_key = 0;
        let mut dec_count_for_versions = -1;
        let mut to_set_version = false;
        let mut version_of_all_keys_in_same_node = 0;
        let mut xmin_vec: Vec<i32> = Vec::new();
        let mut xmax_vec: Vec<Option<u32>> = Vec::new();
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

            // println!("------------------------------------------------");
            // println!("{:?} \n count: {} \n no_of_keys_in_node: {} \n no_of_version_of_all_keys_in_same_node: {}", vec[i], count, no_of_keys_in_node, version_of_all_keys_in_same_node);
            // println!(" dec_count_for_versions: {:?}", dec_count_for_versions);
            // println!("version_of_single_key: {:?}", version_of_single_key);
            // println!("number_of_keys_inspected: {:?}", number_of_keys_inspected);
            // println!("--------------------------------------------------");
            if count == (3 + 5 * version_of_all_keys_in_same_node + no_of_keys_in_node + version_and_key_equal) && count != 3 && version_of_all_keys_in_same_node != version_of_single_key  && no_of_keys_in_node > 0 && number_of_keys_inspected == no_of_keys_in_node {
                version_of_all_keys_in_same_node = 0;
                version_of_single_key = 0;
                no_of_keys_in_node = 0;
                no_of_children = vec[i].to_i32().unwrap();
                vector_deserialized.push(DeserializedNode{ child_count: no_of_children as u32, items: vector_deserialized_items.clone()});
                vector_deserialized_items.clear();
                count = 1;
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
                    version_and_key_equal += 1;
                }
            }
            
            if dec_count_for_versions == 0 {
                // println!("Key {:?}, {}", vec[i], count);
                keys = vec[i].to_i32().unwrap();
                to_set_version = true;
                dec_count_for_versions = -1;
            }
            
            if dec_count_for_versions > 0 && dec_count_for_versions <= 4 * version_of_single_key {
                if dec_count_for_versions % 4 == 0 {
                    // println!("XMIN {:?}", vec[i]);
                    xmin_vec.push(vec[i].to_i32().unwrap());
                } else if (dec_count_for_versions + 1) % 4 == 0 {
                    let result = vec[i].to_i32().unwrap();
                    if result == -1 {
                        xmax_vec.push(None);
                    } else {
                        xmax_vec.push(Some(vec[i].to_i32().unwrap() as u32));
                    }
                    // println!("XMAX {:?}", vec[i]);
                } else if (dec_count_for_versions + 3) % 4 == 0 {
                    values_vec.push(vec[i].to_string().unwrap());
                    // println!("VALUE {:?}", vec[i]);
                }
                
                dec_count_for_versions -= 1;

            } else if dec_count_for_versions > 0 {
                dec_count_for_versions -= 1;
            }
            if dec_count_for_versions == 0 {
                let mut ver_vec:Vec<Version> = Vec::new();
                for j in 0..xmin_vec.len() {
                    ver_vec.push(Version {value: values_vec[j].clone(), xmin: xmin_vec[j] as u32, xmax: xmax_vec[j]});
                }
                values_vec.clear();
                xmin_vec.clear();
                xmax_vec.clear();

                vector_deserialized_items.push(Items {key: keys as u32, rank: node_rank as u32, version: ver_vec.clone() });
                println!("{:?}", vector_deserialized_items);
            }

            if count  == 3  {
                // println!("{:?} {:?}", vec[i], vec[i-1]);
                no_of_keys_in_node = vec[i].to_i32().unwrap();
                node_rank = vec[i-1].to_i32().unwrap();

                dec_count_for_versions = 0;
            }
        }
        // println!("{:?}", vector_deserialized);

        println!("===================================");
        println!("vector_deserialized: {:#?}", vector_deserialized);
        println!("===================================");


        let required_node = vector_deserialized[0].clone();
        let x = Node::deserialized_with_relation(required_node, &mut vector_deserialized);

        let mut k = Node::deserialized_data_to_nodes(x);
        k = Node::deserialized_duplicate_data_check(k);

        let k = Arc::new(RwLock::new(k));
        println!("{:?}", k.read().unwrap().print_tree());
        Ok(k)
        
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

    fn deserialized_duplicate_data_check(mut self_node: Node) -> Node {
        let dup_child_len = self_node.children.len()/2;
        

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
                self_node.children[i] = Arc::new(RwLock::new(Node::deserialized_duplicate_data_check(x)));
            }
        }
        self_node
    }

    fn deserialized_data_to_nodes(deserialized_data: UltraDeserialized) -> Node {
        let mut new_node = Node{input: Vec::new(), rank: 0, children: Vec::new()};

        new_node.input = deserialized_data.parent.items.clone();
        new_node.rank = deserialized_data.parent.items[0].rank;

        if !deserialized_data.children.is_empty() {
            let child_len = deserialized_data.children.len();
            let mut child_vec = Vec::new();
            for j in 0..child_len {
                let k = Arc::new(RwLock::new(Node::deserialized_data_to_nodes(deserialized_data.children[j].clone())));
                child_vec.push(k);
            }
            new_node.children = child_vec;
        }

        new_node
    }

    fn crash_recovery(mut node: Arc<RwLock<Node>>, serialized_file_path: &str, wal_file_path: &str) -> io::Result<(Arc<RwLock<Node>>)> {
/*        let deserialize_result = Node::deserialize(serialized_file_path);
        match deserialize_result {
            Ok(deserialized) => {
                node = deserialized;
            }
            Err(e) => {
                println!("{}", e);
            }
        }
*/
        let mut file = File::open(wal_file_path)?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;
        
        let mut meow = Vec::new();
        for line in contents.lines() {
            let mut k = Vec::new();
            for mut c in line.split_whitespace() {
                c = c.trim_matches('"');
                k.push(c.to_string());
            }
            meow.push(k);
        }
        
        for i in meow.iter() {
            let k = i[1].parse::<u32>().unwrap();
            let z = Arc::clone(&node);
            let result =  Node::key_position(z, k);

            if result.is_none() {
                let s = Arc::clone(&node);
                Node::insert(s, i[1].parse().unwrap(), i[2].clone(), i[0].parse().unwrap()).expect("TODO: panic message");
            }
        }
        
        println!("{:?}", node.read().unwrap().print_tree());
        
        match Node::serialize(Arc::clone(&node)) {
            Ok(_) => {
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(wal_file_path)?;
                file.write_all(b"")?;
            },
            
            Err(e) => {
                println!("{}", e);
            }
        }

        Ok(node)
    }
    
/*    fn checkpoint(mut node: Arc<RwLock<Node>>) -> io::Result<()> {
        let file_path = "/home/_meringue/RustroverProjects/ASMT-V1/WAL.txt";

        match Node::serialize(Arc::clone(&node)) {
            Ok(_) => {
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(file_path)?;

                file.write_all(b"")?;
            },
            Err(e) => {
                println!("{}", e);
            }
        }

        Ok(())
    }*/

    fn wal_updated(file: Arc<RwLock<File>>,k: u32, v: String, wal_file_path: &str) -> io::Result<()> {
        let mut last_lsm = 99;
        match Node::find_last_lsn(wal_file_path) {
            Ok(value) => {
                last_lsm = value;
            }
            Err(e) => {
                println!("{}", e);
            }
        }
        let mut file_instance = file.write().unwrap();

        writeln!(file_instance, "{:?} {:?} {:?}", last_lsm + 1, k, v).expect("TODO: panic message");
        file_instance.sync_all()?;

        COUNTER.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    fn find_last_lsn(wal_file_path: &str) -> io::Result<(u32)> {
        let mut file = File::open(wal_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        println!("{:?}", contents);

        if contents.is_empty() {
            return Ok(99)
        }
        let mut meow = Vec::new();
        for line in contents.lines() {
            let mut k = Vec::new();
            for mut c in line.split_whitespace() {
                c = c.trim_matches('"');
                k.push(c.to_string());
            }
            meow.push(k);
        }

        let mut last_lsm = 1;

        for i in meow.iter() {
            last_lsm = i[0].parse::<u32>().unwrap();
        }

        drop(file);

        Ok(last_lsm)
    }
    
/*    fn wal_immediate_read(node: Arc<RwLock<Node>>, k: u32, wal_file_path: &str) -> io::Result<Option<String>> {
        let mut file = File::open(wal_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut meow = Vec::new();
        for line in contents.lines() {
            let mut k = Vec::new();
            for mut c in line.split_whitespace() {
                c = c.trim_matches('"');
                k.push(c.to_string());
            }
            meow.push(k);
        }

        for i in meow.iter() {
            let wal_key = i[1].parse::<u32>().unwrap();

            if wal_key == k {
                return Ok(Some(i[2].to_string()));
            }
        }

        let result = Node::key_position(node,k);

        Ok(result)
    }
*/
    //TODO: Fix deleting.
    fn wal_immediate_delete(node: Arc<RwLock<Node>>, key: u32, wal_file_path: &str) -> io::Result<()> {
        let mut file = File::open(wal_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(wal_file_path)?;

        for line in contents.lines() {
            let mut k = Vec::new();
            for mut c in line.split_whitespace() {
                c = c.trim_matches('"');
                k.push(c.to_string());
            }
            
            if k[1].parse::<u32>().unwrap() != key {
                writeln!(file, "{:?}", contents)?;
            } else {
                println!("Deleted the key at LSM {:?}", k[0]);
            }
        }
        
        Node::remove_key(Arc::clone(&node), key);
        
        Ok(())
    }
}

impl I32OrString {
    fn to_i32(&self) -> Option<i32> {
        match self {
            I32OrString::Num(num) => Some(*num),
            I32OrString::Str(_) => None,
        }
    }
    
    fn to_string(&self) -> Option<String> {
        match self {
            I32OrString::Num(_) => None,
            I32OrString::Str(str) => Some(str.to_string()),
        }
    }
}

#[derive(Parser)]
#[command(name = "WAT")]
#[command(about = "WATERMELON")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Insert {
        key: u32,
        value: String,
    },
    Push,
    Get {
        key: u32,
    },
    Tree,
    Stats,
}

fn main() -> io::Result<()> {
    NODE_SIZE.set(4).expect("Failed to set size");
    let serialized_file_path = "/home/_meringue/RustroverProjects/ASMT-V1/example.txt";
    let wal_file_path = "/home/_meringue/RustroverProjects/ASMT-V1/WAL.txt";
    let mut new_node = Node::new();


    Node::insert(Arc::clone(&new_node), 1, String::from("Woof"), 1);
    Node::insert(Arc::clone(&new_node), 2, String::from("Woof"), 2);
    Node::insert(Arc::clone(&new_node), 5, String::from("Woof"), 3);
    Node::insert(Arc::clone(&new_node), 15, String::from("Woof"), 4);
    Node::insert(Arc::clone(&new_node), 6, String::from("Woof"), 5);
    Node::insert(Arc::clone(&new_node), 2, String::from("Neigh"), 6);
    Node::insert(Arc::clone(&new_node), 5, String::from("KawKaw"), 7);
    Node::insert(Arc::clone(&new_node), 15, String::from("Quack"), 8);
    Node::insert(Arc::clone(&new_node), 7, String::from("Meow"), 9);
    Node::insert(Arc::clone(&new_node), 8, String::from("Meow"), 10);
    Node::insert(Arc::clone(&new_node), 9, String::from("Meow"), 11);
    // Node::insert(Arc::clone(&new_node), 3, String::from("Meow"), 12);

    println!("{:?}", new_node.read().unwrap().print_tree());
    
    Node::serialize(new_node);
    Node::deserialize(serialized_file_path);
    /*    match Node::deserialize(serialized_file_path) {
            Ok(node) => {
                new_node = node;
            }
            Err(e) => {
                println!("{}", e);
            }
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("/home/_meringue/RustroverProjects/ASMT-V1/WAL.txt")?;
        let file = Arc::new(RwLock::new(file));

        println!("CLI!");
        println!("Enter 'Help' for available commands & 'exit' to quit.");

        let cloned_node = Arc::clone(&new_node);

        let (tx, rx) = mpsc::channel();
        let (sender, receiver) = mpsc::channel();
        let t1 = thread::spawn(move || {
            while let Ok(_) = rx.recv() {
                match push_to_memory(Arc::clone(&cloned_node),serialized_file_path, wal_file_path) {
                    Ok(output_node) => {
                        sender.send(output_node).unwrap();
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                }
            }
        });


        loop {
            print!(">  ");
            io::stdout().flush()?;

            let mut cli_input = String::new();

            match io::stdin().read_line(&mut cli_input) {
                Ok(_) => {
                    let cli_input = cli_input.trim();

                    if cli_input.is_empty() {
                        continue;
                    }

                    let args = cli_input.split_whitespace().collect::<Vec<&str>>();

                    if args.is_empty() {
                        continue;
                    }

                    match args[0].to_lowercase().as_str() {
                        "insert" => {
                            if args.len() != 3 {
                                println!("Invalid argument");
                                continue;
                            }

                            let key = args[1].parse::<u32>().expect("Invalid argument");
                            let value = args[2].parse::<String>().expect("Invalid argument");

                            println!("Inserting key {}", key);
                            Node::wal_updated(Arc::clone(&file), key, value, wal_file_path)?;
                            CHECKPOINT_COUNTER.fetch_add(1, Ordering::Relaxed);
                            println!("Inserted");
                        }

                        "push" => {
                            if args.len() != 1 {
                                println!("Invalid argument");
                                continue;
                            }
                            tx.send(1).unwrap();
                            new_node = receiver.recv().unwrap();
                        }

                        "get" => {
                            if args.len() != 2 {
                                println!("Invalid argument");
                                continue;
                            }

                            let key = args[1].parse::<u32>().expect("Invalid argument");

                            match Node::wal_immediate_read(Arc::clone(&new_node), key, wal_file_path) {
                                Ok(Some(value)) => {
                                    println!("{}", value);
                                }
                                Ok(None) => {
                                    println!("No value found");
                                }
                                Err(e) => {
                                    println!("{}", e);
                                }
                            }
                        }

                        "delete" => {
                            if args.len() != 2 {
                                println!("Invalid argument");
                                continue;
                            }

                            let key = args[1].parse::<u32>().expect("Invalid argument");

                            Node::wal_immediate_delete(Arc::clone(&new_node), key, wal_file_path)?;
                        }

                        "tree" => {
                            if args.len() != 1 {
                                println!("Invalid argument");
                                continue;
                            }

                            println!("{:?}", new_node.read().unwrap().print_tree());
                        }

                        "stats" => {
                            if args.len() != 1 {
                                println!("Invalid argument");
                                continue;
                            }

                            println!("{:?}", new_node.read().unwrap().print_stats());
                        }

                        "help" => {
                            if args.len() != 1 {
                                println!("Invalid argument");
                                continue;
                            }

                            println!("  insert <key> <value>  - Insert a key-value pair");
                            println!("  push                  - Push inserted key-value to B-Tree");
                            println!("  get <key>             - Get value for a key");
                            println!("  delete <key>          - Delete a key (Broken Sorry)");
                            println!("  tree                  - Show B-Tree in ASCII art form");
                            println!("  stats                 - Show B-Tree Stats");
                            println!("  help                  - Show this help");
                            println!("  exit                  - Exit the program");
                        }

                        "exit" => {
                            if args.len() != 1 {
                                println!("Invalid argument");
                                continue;
                            }

                            println!("Exiting");
                            break;
                        }

                        _ => {
                            println!("Unknown command: {}. Type 'help' for available commands.", args[0]);
                        }
                    }

                    let metadata = fs::metadata(wal_file_path)?;
                    let size = metadata.len();

                    if CHECKPOINT_COUNTER.load(Ordering::Relaxed) >= 100 && size >= 1024 {
                        println!("Maximum WAL file size exceeded.");
                        tx.send(1).unwrap();
                    }
                }
                Err(e) => {
                    println!("Invalid argument. Error: {:?}",e );;
                }
            }
        }

        t1.join().unwrap();*/

    Ok(())
}

fn push_to_memory(node: Arc<RwLock<Node>>, serialized_file_path: &str, wal_file_path: &str) -> io::Result<Arc<RwLock<Node>>> {
    println!("Pushing disk values to in-memory B-Tree");
    let returned_node = Node::crash_recovery(node,serialized_file_path, wal_file_path);
    CHECKPOINT_COUNTER.store(0, Ordering::Relaxed);
    println!("Pushed disk values");
    returned_node

}

impl Node {
    /// Pretty print the entire tree starting from this node
    pub fn print_tree(&self) {
        self.print_tree_recursive("", true, 0, None);
    }

    /// Pretty print the tree with transaction visibility
    pub fn print_tree_for_transaction(&self, tx_id: u32) {
        self.print_tree_recursive("", true, 0, Some(tx_id));
    }

    /// Recursive helper for tree printing
    fn print_tree_recursive(&self, prefix: &str, is_last: bool, depth: usize, tx_id: Option<u32>) {
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
                 self.format_items(tx_id));

        // Prepare prefix for children
        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        // Print children
        for (i, child_arc) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;

            match child_arc.read() {
                Ok(child) => {
                    child.print_tree_recursive(&child_prefix, is_last_child, depth + 1, tx_id);
                }
                Err(_) => {
                    println!("{}{}[POISONED RWLOCK]",
                             child_prefix,
                             if is_last_child { "└── " } else { "├── " });
                }
            }
        }
    }

    fn format_items(&self, tx_id: Option<u32>) -> String {
        if self.input.is_empty() {
            return "empty".to_string();
        }

        let items: Vec<String> = self.input
            .iter()
            .map(|item| {
                let visible_versions = if let Some(tx) = tx_id {
                    self.format_visible_versions(&item.version, tx)
                } else {
                    self.format_all_versions(&item.version)
                };

                format!("{}:{} (rank: {})", item.key, visible_versions, item.rank)
            })
            .collect();

        items.join(", ")
    }

    fn format_all_versions(&self, versions: &[Version]) -> String {
        if versions.is_empty() {
            return "[]".to_string();
        }

        let version_strs: Vec<String> = versions
            .iter()
            .map(|v| {
                let xmax_str = match v.xmax {
                    Some(xmax) => format!("{}", xmax),
                    None => "∞".to_string(),
                };
                format!("{}[{}-{}]", v.value, v.xmin, xmax_str)
            })
            .collect();

        format!("[{}]", version_strs.join(", "))
    }

    fn format_visible_versions(&self, versions: &[Version], tx_id: u32) -> String {
        let visible_versions: Vec<&Version> = versions
            .iter()
            .filter(|v| self.is_version_visible(v, tx_id))
            .collect();

        if visible_versions.is_empty() {
            return "[DELETED]".to_string();
        }

        // Get the most recent visible version
        let latest_version = visible_versions
            .iter()
            .max_by_key(|v| v.xmin)
            .unwrap();

        format!("{}", latest_version.value)
    }

    fn is_version_visible(&self, version: &Version, tx_id: u32) -> bool {
        // Version is visible if:
        // 1. It was created before or by this transaction (xmin <= tx_id)
        // 2. It wasn't deleted, or was deleted after this transaction (xmax is None or xmax > tx_id)
        version.xmin <= tx_id && version.xmax.map_or(true, |xmax| xmax > tx_id)
    }

    /// Alternative compact horizontal view
    pub fn print_compact(&self) {
        self.print_compact_for_transaction(None);
    }

    /// Compact view for specific transaction
    pub fn print_compact_for_transaction(&self, tx_id: Option<u32>) {
        println!("B-Tree Structure{}:",
                 tx_id.map(|tx| format!(" (TX: {})", tx)).unwrap_or_default());
        println!("{}", "=".repeat(50));
        self.print_compact_recursive(0, tx_id);
    }

    fn print_compact_recursive(&self, level: usize, tx_id: Option<u32>) {
        let indent = "  ".repeat(level);
        println!("{}Level {}: [{}] (rank: {})",
                 indent,
                 level,
                 self.format_items(tx_id),
                 self.rank);

        for (i, child_arc) in self.children.iter().enumerate() {
            match child_arc.read() {
                Ok(child) => {
                    if i == 0 && !self.children.is_empty() {
                        println!("{}Children:", "  ".repeat(level + 1));
                    }
                    child.print_compact_recursive(level + 1, tx_id);
                }
                Err(_) => {
                    println!("{}[POISONED RWLOCK]", "  ".repeat(level + 1));
                }
            }
        }
    }

    /// Tree statistics
    pub fn print_stats(&self) {
        self.print_stats_for_transaction(None);
    }

    /// Tree statistics for specific transaction
    pub fn print_stats_for_transaction(&self, tx_id: Option<u32>) {
        let stats = self.calculate_stats(tx_id);
        println!("Tree Statistics{}:",
                 tx_id.map(|tx| format!(" (TX: {})", tx)).unwrap_or_default());
        println!("├── Total nodes: {}", stats.total_nodes);
        println!("├── Tree height: {}", stats.height);
        println!("├── Total keys: {}", stats.total_keys);
        println!("├── Visible keys: {}", stats.visible_keys);
        println!("├── Total versions: {}", stats.total_versions);
        println!("├── Leaf nodes: {}", stats.leaf_nodes);
        println!("└── Internal nodes: {}", stats.internal_nodes);
    }

    fn calculate_stats(&self, tx_id: Option<u32>) -> TreeStats {
        let mut stats = TreeStats::default();
        self.calculate_stats_recursive(&mut stats, 0, tx_id);
        stats
    }

    fn calculate_stats_recursive(&self, stats: &mut TreeStats, depth: usize, tx_id: Option<u32>) {
        stats.total_nodes += 1;
        stats.total_keys += self.input.len();
        stats.height = stats.height.max(depth + 1);

        // Count versions and visible keys
        for item in &self.input {
            stats.total_versions += item.version.len();

            if let Some(tx) = tx_id {
                let has_visible_version = item.version
                    .iter()
                    .any(|v| self.is_version_visible(v, tx));
                if has_visible_version {
                    stats.visible_keys += 1;
                }
            } else {
                stats.visible_keys += 1; // All keys are "visible" when no tx specified
            }
        }

        if self.children.is_empty() {
            stats.leaf_nodes += 1;
        } else {
            stats.internal_nodes += 1;
            for child_arc in &self.children {
                if let Ok(child) = child_arc.read() {
                    child.calculate_stats_recursive(stats, depth + 1, tx_id);
                }
            }
        }
    }

    /// Print version history for debugging
    pub fn print_version_history(&self) {
        println!("Version History:");
        println!("{}", "=".repeat(60));
        self.print_version_history_recursive("", true, 0);
    }

    fn print_version_history_recursive(&self, prefix: &str, is_last: bool, depth: usize) {
        let connector = if depth == 0 {
            "Root"
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        println!("{}{}Node(rank: {})", prefix, connector, self.rank);

        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        // Print detailed version info for each item
        for (i, item) in self.input.iter().enumerate() {
            let item_connector = if i == self.input.len() - 1 && self.children.is_empty() {
                "└── "
            } else {
                "├── "
            };

            println!("{}{}Key {}: (rank: {})", child_prefix, item_connector, item.key, item.rank);

            for (v_idx, version) in item.version.iter().enumerate() {
                let version_connector = if v_idx == item.version.len() - 1 {
                    "    └── "
                } else {
                    "    ├── "
                };

                let xmax_str = match version.xmax {
                    Some(xmax) => format!("{}", xmax),
                    None => "∞".to_string(),
                };

                println!("{}{}\"{}\" [TX {}-{}]",
                         child_prefix, version_connector,
                         version.value, version.xmin, xmax_str);
            }
        }

        // Print children
        for (i, child_arc) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;

            match child_arc.read() {
                Ok(child) => {
                    child.print_version_history_recursive(&child_prefix, is_last_child, depth + 1);
                }
                Err(_) => {
                    println!("{}{}[POISONED RWLOCK]",
                             child_prefix,
                             if is_last_child { "└── " } else { "├── " });
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
    visible_keys: usize,
    total_versions: usize,
    leaf_nodes: usize,
    internal_nodes: usize,
}