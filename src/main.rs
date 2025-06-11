use std::sync::{Arc, Mutex};
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
}

static NODE_INSTANCE: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));


impl Node {
    fn new() -> Arc<Mutex<Node>> {
        NODE_INSTANCE.fetch_add(1, Ordering::SeqCst);
        let instance = Arc::new(Mutex::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
        }));
        // NODE_INSTANCE.lock().unwrap().push(instance.clone());
        instance
    }

    fn insert(&mut self, k: u32, v: String) -> () {
        if !self.children.is_empty() {
            self.add_child_key(Items {key: k, value: v.clone(), rank: self.rank});
        }
        else {
            self.input.push(Items {key: k, value: v, rank: self.rank});

        }

        self.overflow_check();

        self.min_size_check(Node::new().lock().unwrap().clone());
        self.sort_main_nodes();

        self.overflow_check();
        self.tree_split_check();
    }

    fn overflow_check(&mut self) -> () {
        if self.input.len() > *NODE_SIZE.get().unwrap() {
            self.split_nodes();
        } else if !self.children.is_empty() {
            for i in 0..self.children.len() {
                self.children[i].lock().unwrap().overflow_check();
            }
        }
    }

    fn split_nodes(&mut self) -> () {
        self.sort_main_nodes();

        let struct_one = Node::new();
        let struct_two = Node::new();

        let items_size = self.input.len();
        let breaking_point = (items_size + 1)/2;
        let temp_storage = self.clone();
        let mut count = 0;
        let mut i = 0;
        self.input.clear();
        for _v in temp_storage.input.iter() {
            count +=1;

            if count == breaking_point {
                self.input.push(temp_storage.input[count-1].clone());
            } else if count > breaking_point {
                i = i + 1;
                struct_two.lock().unwrap().input.push(temp_storage.input[count - 1].clone());
                struct_two.lock().unwrap().input[i - 1].rank = temp_storage.rank + 1;
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone());
                struct_one.lock().unwrap().input[count - 1].rank = temp_storage.rank + 1;
            }
        }


        struct_one.lock().unwrap().rank = self.rank + 1;
        struct_two.lock().unwrap().rank = self.rank + 1;
        self.children.push(struct_one.clone());
        self.children.push(struct_two.clone());
    }

    fn tree_split_check(&mut self) -> () {
        if self.input.len() + 1 != self.children.len() && !self.children.is_empty() {
            self.merge_weird_splitting();
        } else if !self.children.is_empty() {
            for i in 0..self.children.len() {
                self.children[i].lock().unwrap().tree_split_check();
            }
        }
    }

    fn merge_weird_splitting(&mut self) -> () {
        println!("merge_weird_splitting {:?}", self);

        let x = self.children[self.children.len()-2].lock().unwrap().input.len();
        let y = self.children[self.children.len()-1].lock().unwrap().input.len();

        for i in 0..x+1 {
            self.rank_correction();
            self.children[self.children.len()-2].lock().unwrap().children.push(self.children[0].clone());
            self.children.remove(0);
        }

        for i in 0..y+1 {
            self.rank_correction();
            self.children[self.children.len()-1].lock().unwrap().children.push(self.children[0].clone());
            self.children.remove(0);
        }
        
    }
    fn rank_correction(&mut self) {
        self.children[0].lock().unwrap().rank = self.children[self.children.len()-1].lock().unwrap().rank + 1;
        let k = self.children[0].lock().unwrap().input.len();
        for j in 0..k {
            self.children[0].lock().unwrap().input[j].rank = self.children[self.children.len()-1].lock().unwrap().rank + 1;
        }
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

    fn min_size_check(&mut self, z: Node) -> () {
        let meow = self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>();

        for j in meow.clone() {
            let mut jj = j.lock().unwrap().clone();
            if jj.input.len() < *NODE_SIZE.get().unwrap() / 2 && jj.rank > 1 {
                self.propagate_up(jj.clone());
            } else if !jj.children.is_empty() {
                for i in 0..jj.children.len() {
                    jj.children[i].lock().unwrap().min_size_check(jj.clone());
                }
            }
        }

    }

    fn propagate_up(&mut self, mut child: Node) {
        for i in 0..child.input.len() {
            child.input[i].rank = self.input[0].rank;
            self.input.push(child.input[i].clone());
        }
        for i in 0..child.children.len() {
            child.children[i].lock().unwrap().rank = self.children[0].lock().unwrap().rank;
            let k = child.children[i].lock().unwrap().input.len();

            for j in 0..k {
                child.children[i].lock().unwrap().input[j].rank = self.children[0].lock().unwrap().rank;
            }

            self.children.push(child.children[i].clone());
        }

        for i in 0..self.children.len() - 1 {
            if self.children[i].lock().unwrap().input[0].key == child.input[0].key {
                self.children.remove(i);
            }
        }
        self.sort_children_nodes();
    }


    fn sort_children_nodes(&mut self) {
        self.children.sort_by(|a, b| {a.lock().unwrap().input[0].key.cmp(&b.lock().unwrap().input[0].key)});
    }
    fn sort_main_nodes(&mut self) {
        self.input.sort_by(|a, b| {a.key.cmp(&b.key)});
    }
    fn sort_children_items(&mut self) {
        for i in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
            i.lock().unwrap().input.sort_by(|a,b| {a.key.cmp(&b.key)});
        }
    }


}

fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let f = Node::new();
    f.lock().unwrap().insert(100, String::from("a"));
    f.lock().unwrap().insert(400, String::from("Squeak"));
    f.lock().unwrap().insert(200, String::from("Woof"));
    f.lock().unwrap().insert(450, String::from("Meow"));
    f.lock().unwrap().insert(300, String::from("Caw-Caw"));
    f.lock().unwrap().insert(130, String::from("Chirp"));
    f.lock().unwrap().insert(430, String::from("Ribbit"));
    f.lock().unwrap().insert(420, String::from("Purr"));
    f.lock().unwrap().insert(180, String::from("Neigh"));
    f.lock().unwrap().insert(70, String::from("Myahhh"));
    f.lock().unwrap().insert(500, String::from("Oink-Oink"));
    f.lock().unwrap().insert(210, String::from("a"));
    f.lock().unwrap().insert(280, String::from("c"));
    f.lock().unwrap().insert(320, String::from("Bah"));
    f.lock().unwrap().insert(410, String::from("X"));
    f.lock().unwrap().insert(460, String::from("K"));
    f.lock().unwrap().insert(480, String::from("Hi"));
    f.lock().unwrap().insert(490, String::from("Hiss"));
    f.lock().unwrap().insert(440, String::from("Ha"));
    f.lock().unwrap().insert(470, String::from("q"));
    f.lock().unwrap().insert(120, String::from("Mahh"));
    f.lock().unwrap().insert(40, String::from("Howl"));
    f.lock().unwrap().insert(520, String::from("dsad"));
    f.lock().unwrap().insert(600, String::from("juityuy"));
    f.lock().unwrap().insert(620, String::from("Bau Bau"));

    //ToDO: Combine the newly added keys to required child vector
    println!("--------------------------------------------------------------------------------");
    println!("A  {:?}", f.try_lock().unwrap().input);
    println!("--------------------------------------------------------------------------------");
    println!("B  {:?}", f.try_lock().unwrap().children);
    println!("--------------------------------------------------------------------------------");
    println!("A  {:?}", f);

    println!("{:?}", f.lock().unwrap().print_tree());

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
