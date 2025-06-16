extern crate rand;

use std::sync::{Arc, Mutex, MutexGuard, Weak};
use once_cell::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};


static NODE_SIZE: OnceCell<usize> = OnceCell::new();

#[derive(Debug, Clone, PartialEq)]
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
    parent: Option<Weak<Mutex<Node>>>,
}

static NODE_INSTANCE: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));


impl Node {
    fn new() -> Arc<Mutex<Node>> {
        NODE_INSTANCE.fetch_add(1, Ordering::SeqCst);
        let instance = Arc::new(Mutex::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
            parent: None,
        }));
        instance
    }

    fn insert(self_node: &mut Arc<Mutex<Node>>, k: u32, v: String) -> () {
        let mut z = self_node.try_lock().unwrap();
        let rank = z.rank;
        if !z.children.is_empty() {
            z.add_child_key(Items {key: k, value: v.clone(), rank});
        }
        else {
            z.input.push(Items {key: k, value: v, rank});
        }

        z = Node::overflow_check(z);

        z = Node::min_size_check(z);
        z.sort_main_nodes();

        z = Node::tree_split_check(z);

        z = Node::min_size_check(z);

        z = Node::overflow_check(z);

        z = Node::min_size_check(z);
        
        z.sort_main_nodes();
        
        z = Node::tree_split_check(z);

        // z = Node::min_size_check(z);

        // z = Node::tree_split_check(z);

    }

    fn overflow_check(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut x = self_node;
        if x.input.len() > *NODE_SIZE.get().unwrap() {
            x = Node::split_nodes(x);
        } else if !x.children.is_empty() {
            for i in 0..x.children.len() {
                let _unused = Node::overflow_check(x.children[i].lock().unwrap());
            }
        }
        x
    }

    fn split_nodes(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;

        self_instance.sort_main_nodes();

        let struct_one = Node::new();
        let struct_two = Node::new();


        let items_size = self_instance.input.len();
        let breaking_point = (items_size + 1)/2;
        let temp_storage = self_instance.clone();
        let mut count = 0;
        let mut i = 0;
        self_instance.input.clear();
        for _v in temp_storage.input.iter() {
            count +=1;

            if count == breaking_point {
                self_instance.input.push(temp_storage.input[count-1].clone());
            } else if count > breaking_point {
                i = i + 1;
                struct_two.lock().unwrap().input.push(temp_storage.input[count - 1].clone());
                struct_two.lock().unwrap().input[i - 1].rank = temp_storage.rank + 1;
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone());
                struct_one.lock().unwrap().input[count - 1].rank = temp_storage.rank + 1;
            }
        }


        struct_one.lock().unwrap().rank = self_instance.rank + 1;
        struct_two.lock().unwrap().rank = self_instance.rank + 1;
        
        // struct_one.lock().unwrap().parent = Some(Arc::downgrade(&Arc::new(self_instance)));
        self_instance.children.push(struct_one.clone());
        self_instance.children.push(struct_two.clone());
        
        // self.min_size_check();

        self_instance
    }

    fn tree_split_check(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;
        if self_instance.input.len() + 1 != self_instance.children.len() && !self_instance.children.is_empty() {
            self_instance = Node::merge_weird_splitting(self_instance);

        } else if !self_instance.children.is_empty() {
            for i in 0..self_instance.children.len() {
                Node::tree_split_check(self_instance.children[i].lock().unwrap());
            }
        }

        self_instance
    }

    fn merge_weird_splitting(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut self_instance = self_node;
        // HACK: Go through all instance of node and pick thenode input where node whose lowest value is lesser than X's lowest val & 
        // highest value is higher than X's highest value. It should be duplicated and removed. And let the following function progress.
        
        let child_len = self_instance.children.len();
        for i in 0..child_len {
            let some_val = self_instance.children[i].lock().unwrap().clone().input;
            let k = self_instance.children[i].clone();
            let keys_primary_required = vec![some_val[0].key, some_val[some_val.len() - 1].key];
            
            for j in 0..child_len {
                let some_other_val = self_instance.children[j].lock().unwrap().clone().input;
                let keys_secondary_required = vec![some_other_val[0].key, some_other_val[some_other_val.len() - 1].key];

                if keys_primary_required[0] < keys_secondary_required[0] && keys_primary_required[1] > keys_secondary_required[1] {
                    self_instance.children.push(k.clone());
                    self_instance.children.remove(i);
                    break
                }
            }
        }

        let len = self_instance.children.len();
        if len >= 2 {
            let last_two = &mut self_instance.children[len-2..];
            last_two.sort_by(|a, b| { a.lock().unwrap().input[0].key.cmp(&b.lock().unwrap().input[0].key) });
        }
        
        println!("{:?}", self_instance.print_tree());
        
        let x = self_instance.children[self_instance.children.len()-2].lock().unwrap().input.len();
        let y = self_instance.children[self_instance.children.len()-1].lock().unwrap().input.len();

        // HACK: only selected the children with no children of their own to be added under new node.
        for _i in 0..x+1 {
            Node::rank_correction(&mut self_instance);
            let mut p = 0;
            while !self_instance.children[p].lock().unwrap().children.is_empty() {
                p = p + 1;
            }
            
            self_instance.children[self_instance.children.len()-2].lock().unwrap().children.push(self_instance.children[p].clone());
            self_instance.children.remove(p);

        }

        for _i in 0..y+1 {
            Node::rank_correction(&mut self_instance);
            let mut p = 0;
            while !self_instance.children[p].lock().unwrap().children.is_empty() {
                p = p + 1;
            }
            self_instance.children[self_instance.children.len()-1].lock().unwrap().children.push(self_instance.children[p].clone());
            self_instance.children.remove(p);
        }

        self_instance
    }
    fn rank_correction(self_instance: &mut MutexGuard<Node>) {
/*        let child_size = self_instance.children.len();
        let rank_tbc = self_instance.children[self_instance.children.len()-1].lock().unwrap().rank + 1;
        // let rank_tbc = self_instance.rank + 2;
        println!("rank_tbc: {}", rank_tbc);
        for i in 0..child_size {
            self_instance.children[i].lock().unwrap().rank = rank_tbc;
            let k = self_instance.children[i].lock().unwrap().input.len();
            for j in 0..k {
                self_instance.children[i].lock().unwrap().input[j].rank = rank_tbc;
            }
        }*/

        self_instance.children[0].lock().unwrap().rank = self_instance.children[self_instance.children.len()-1].lock().unwrap().rank + 1;
        let k = self_instance.children[0].lock().unwrap().input.len();
        for j in 0..k {
            self_instance.children[0].lock().unwrap().input[j].rank = self_instance.children[self_instance.children.len()-1].lock().unwrap().rank + 1;
        }
    }

/*    fn min_size_subceeded_check(&mut self) -> () {
/*        if !self.children.is_empty() {
            for i in 0..self.children.len() {
                let x = self.children[i].lock().unwrap().input.len();
                if self.children[i].lock().unwrap().input.len() < *NODE_SIZE.get().unwrap()/2 {
                    self.min_size_subceeded(i);
                }
            }
        }*/

        if self.input.len() < *NODE_SIZE.get().unwrap()/2 && self.rank != 1 {
            self.min_size_subceeded();
        }
    }

    fn min_size_subceeded(&mut self, i: usize) -> () {
        println!("min_size_subceeded");
        self.children[i].lock().unwrap().rank = self.rank;
        let child_length = self.children[i].lock().unwrap().children.len();

        for j in 0..child_length {
            self.children[i].lock().unwrap().children[j].lock().unwrap().rank = self.input[0].rank;
        }
    }*/

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

    fn min_size_check(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut x = self_node;

        let mut indices_to_propagate = Vec::new();
        for (idx, child) in x.children.iter().enumerate() {
            let child_lock = child.lock().unwrap();
            if child_lock.input.len() < *NODE_SIZE.get().unwrap() / 2 && child_lock.rank > 1 {
                indices_to_propagate.push(idx);
            }
        }

        for &idx in indices_to_propagate.iter().rev() {
            let child_clone = x.children[idx].lock().unwrap().clone();
            x = Node::propagate_up(x, child_clone);
        }

        for child in &x.children {
            let mut child_lock = child.lock().unwrap();
            if !child_lock.children.is_empty() {
                Node::min_size_check(child_lock);
            }
        }
        
        x
    }

    fn propagate_up(self_node: MutexGuard<Node>, mut child: Node) -> MutexGuard<Node> {
        let mut x = self_node;
        for i in 0..child.input.len() {
            child.input[i].rank = x.input[0].rank;
            x.input.push(child.input[i].clone());
        }
        for i in 0..child.children.len() {
            child.children[i].lock().unwrap().rank = x.children[0].lock().unwrap().rank;
            let k = child.children[i].lock().unwrap().input.len();

            for j in 0..k {
                child.children[i].lock().unwrap().input[j].rank = x.children[0].lock().unwrap().rank;
            }

            x.children.push(child.children[i].clone());
        }

        for i in 0..x.children.len() - 1 {
            if x.children[i].lock().unwrap().input[0].key == child.input[0].key {
                x.children.remove(i);
            }
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

}

// TODO: Fix some child nodes having lower size than min size.
fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let mut f = Node::new();

    Node::insert(&mut f,127, String::from("Woof"));

    Node::insert(&mut f, 127, String::from("Woof"));
    Node::insert(&mut f, 543, String::from("Woof"));
    Node::insert(&mut f, 89, String::from("Woof"));
    Node::insert(&mut f, 312, String::from("Woof"));
    Node::insert(&mut f, 476, String::from("Woof"));
    Node::insert(&mut f, 25, String::from("Woof"));
    Node::insert(&mut f, 598, String::from("Woof"));
    Node::insert(&mut f, 341, String::from("Woof"));
    Node::insert(&mut f, 67, String::from("Woof"));
    Node::insert(&mut f, 429, String::from("Woof"));
    Node::insert(&mut f, 182, String::from("Woof"));
    Node::insert(&mut f, 564, String::from("Woof"));
    Node::insert(&mut f, 203, String::from("Woof"));
    Node::insert(&mut f, 497, String::from("Woof"));
    Node::insert(&mut f, 38, String::from("Woof"));
    Node::insert(&mut f, 621, String::from("Woof"));
    Node::insert(&mut f, 154, String::from("Woof"));
    Node::insert(&mut f, 287, String::from("Woof"));
    Node::insert(&mut f, 453, String::from("Woof"));
    Node::insert(&mut f, 72, String::from("Woof"));
    Node::insert(&mut f, 509, String::from("Woof"));
    Node::insert(&mut f, 236, String::from("Woof"));
    Node::insert(&mut f, 375, String::from("Woof"));
    Node::insert(&mut f, 418, String::from("Woof"));
    Node::insert(&mut f, 95, String::from("Woof"));
    Node::insert(&mut f, 582, String::from("Woof"));
    Node::insert(&mut f, 167, String::from("Woof"));
    Node::insert(&mut f, 324, String::from("Woof"));
    Node::insert(&mut f, 491, String::from("Woof"));
    Node::insert(&mut f, 53, String::from("Woof"));
    Node::insert(&mut f, 17, String::from("Woof"));
    Node::insert(&mut f, 248, String::from("Woof"));
    Node::insert(&mut f, 399, String::from("Woof"));
    Node::insert(&mut f, 521, String::from("Woof"));
    Node::insert(&mut f, 64, String::from("Woof"));
    Node::insert(&mut f, 192, String::from("Woof"));
    Node::insert(&mut f, 355, String::from("Woof"));
    // Node::insert(&mut f, 478, String::from("Woof"));
    // Node::insert(&mut f, 106, String::from("Woof"));
    // Node::insert(&mut f, 273, String::from("Woof"));
    // Node::insert(&mut f, 412, String::from("Woof"));
    // Node::insert(&mut f, 539, String::from("Woof"));
    // Node::insert(&mut f, 81, String::from("Woof"));
    // Node::insert(&mut f, 226, String::from("Woof"));
    // Node::insert(&mut f, 367, String::from("Woof"));
    // Node::insert(&mut f, 504, String::from("Woof"));
    // Node::insert(&mut f, 143, String::from("Woof"));
    // Node::insert(&mut f, 289, String::from("Woof"));
    // Node::insert(&mut f, 432, String::from("Woof"));
    // Node::insert(&mut f, 619, String::from("Woof"));

    println!("{:?}", f.lock().unwrap().print_tree());
/*    println!("{:?}", f.lock().unwrap().print_compact());
    println!("{:?}", f.lock().unwrap().print_stats());
*/
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
