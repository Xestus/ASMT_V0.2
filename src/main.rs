extern crate rand;

use std::io;
use std::ptr::read;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use once_cell::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::Rng;

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
        z.sort_children_nodes();
        z = Node::tree_split_check(z);
        z = Node::min_size_check(z);

        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z.sort_main_nodes();
        z = Node::tree_split_check(z);
        z = Node::rank_correction(z);
        z.sort_everything();
        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z = Node::tree_split_check(z);
        z = Node::rank_correction(z);
        z.sort_everything();

    }
    fn overflow_check(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut x = self_node;
        if x.input.len() > *NODE_SIZE.get().unwrap() {
            x = Node::split_nodes(x);
        } else if !x.children.is_empty() {
            for i in 0..x.children.len() {
                Node::overflow_check(x.children[i].lock().unwrap());
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
        
        self_instance.children.push(struct_one.clone());
        self_instance.children.push(struct_two.clone());
        
        
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
        
        // TODO: So the issue is that the nodes split and take pre existing nodes. Hence, create a iterator which goes thru the required number and parent node, selects the count where it lands.
        // For eg: Rank 1 node: [0] 643 [1] 1023 [2]. If the value of the new rank 2 node falls between 643 & 1023 i.e. count [1], it takes all of its siblings which fall on the same range,.

        let mut self_instance = self_node;
        // HACK: Go through all instance of node and pick the node input where node whose lowest value is lesser than X's lowest val &
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

        let mut required_child = vec![self_instance.children[self_instance.children.len()-2].lock().unwrap().clone()];
        required_child.push(self_instance.children[self_instance.children.len()-1].lock().unwrap().clone());

        let x = self_instance.children[self_instance.children.len()-2].lock().unwrap().input.len();
        let y = self_instance.children[self_instance.children.len()-1].lock().unwrap().input.len();

        // HACK: only selected the children with no children of their own to be added under new node.
        // TODO: Hank no work as most of the time, you need to merge childrens with childrens.

        let mut for_x = Vec::new();
        let mut for_y = Vec::new();

        for i in 0..2 {
            let mut holder = Vec::new();
            let k = [x, y][i];
            if required_child[i].input[k-1].key < self_instance.input[0].key {
                holder = vec![0, self_instance.input[0].key]
            } else if required_child[i].input[0].key > self_instance.input[self_instance.input.len() - 1].key {
                holder = vec![self_instance.input[self_instance.input.len() - 1].key, 1000000000]
            } else {
                for j in 0..self_instance.input.len()-1 {
                    if required_child[i].input[0].key > self_instance.input[j].key && required_child[i].input[k-1].key < self_instance.input[j+1].key {
                        holder =vec![self_instance.input[j].key, self_instance.input[j+1].key]
                    }
                }
            }
            match i {
                0 => for_x = holder,
                1 => for_y = holder,
                _ => {}
            }
        }

        let mut j = 0;
        for _i in 0..self_instance.children.len() - 2 {
            let k = self_instance.children[j].lock().unwrap().clone();
            if k.input[0].key > for_x[0] && k.input[k.input.len() - 1].key < for_x[1] {

                self_instance.children[self_instance.children.len()-2].lock().unwrap().children.push(Arc::new(Mutex::new(k)));
                self_instance.children.remove(j);
            } else if k.input[0].key > for_y[0] && k.input[k.input.len() - 1].key < for_y[1] {
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
                Node::rank_correction(self_instance.children[i].lock().unwrap());
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
    fn sort_everything(&mut self) {
        self.sort_main_nodes();
        self.sort_children_nodes();

        let children: Vec<Arc<Mutex<Node>>> = self.children.clone();

        for child in children {
            let mut child_guard = child.lock().unwrap();
            child_guard.sort_everything();
        }
    }

    fn key_position(node: Arc<Mutex<Node>>, key: u32) -> Option<Items> {
        let mut node_instance = node.lock().unwrap();
        for i in 0..node_instance.input.len() {
            if node_instance.input[i].key == key {
                return Some(node_instance.input[i].clone());
            }
        }

        if key < node_instance.input[0].key {
            return Node::key_position(node_instance.children[0].clone(), key);
        } else if key > node_instance.input[node_instance.input.len()-1].key {
            return Node::key_position(node_instance.children[node_instance.children.len()-1].clone(), key);
        } else {
            for i in 0..node_instance.input.len() - 1 {
                if key > node_instance.input[i].key && key < node_instance.input[i+1].key {
                    return Node::key_position(node_instance.children[i+1].clone(), key);
                }
            }
        }

        None
    }
    
    fn remove_key(self_node: &mut Arc<Mutex<Node>>, key: u32) {
        Node::remove_key_extension(self_node, key);
        let mut x = self_node.lock().unwrap();
        x = Node::removed_node_check(x);
        // x.sort_everything();

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
            let child_clone = x.children[idx].lock().unwrap().clone();
            x = Node::parent_key_down(x, child_clone, idx);
        }

        for child in &x.children {
            let mut child_lock = child.lock().unwrap();
            if !child_lock.children.is_empty() {
                Node::removed_node_check(child_lock);
            }
        }

        x
    }
    
    fn parent_key_down(self_node: MutexGuard<Node>, mut child: Node, idx: usize) -> MutexGuard<Node> {
        let mut self_instance = self_node;
        println!("{:?} \n {:?} \n {}", self_instance.print_tree(), child.print_tree(), idx);
        
        let break_point = (self_instance.children.len() + 1) / 2;
        
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

        println!("{:?}", self_instance.print_tree());

        self_instance
    }
}

fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let mut f = Node::new();
    let mut c = 0;
/*    for i in 0..100 {
        let sec = rand::thread_rng().gen_range(1, 1000);
        Node::insert(&mut f, sec, String::from("Woof"));
        c = c + 1;
        println!("{} - {}", c, sec);
    }*/

    Node::insert(&mut f, 296, String::from("Woof"));
    Node::insert(&mut f, 574, String::from("Woof"));
    Node::insert(&mut f, 511, String::from("Woof"));
    Node::insert(&mut f, 483, String::from("Woof"));
    Node::insert(&mut f, 43, String::from("Woof"));
    Node::insert(&mut f, 182, String::from("Woof"));
    Node::insert(&mut f, 597, String::from("Woof"));
    Node::insert(&mut f, 482, String::from("Woof"));
    Node::insert(&mut f, 767, String::from("Woof"));
    Node::insert(&mut f, 514, String::from("Woof"));
    Node::insert(&mut f, 137, String::from("Woof"));
    Node::insert(&mut f, 842, String::from("Woof"));
    Node::insert(&mut f, 148, String::from("Woof"));
    Node::insert(&mut f, 3, String::from("Woof"));
    Node::insert(&mut f, 687, String::from("Woof"));
    Node::insert(&mut f, 292, String::from("Woof"));
    Node::insert(&mut f, 320, String::from("Woof"));
    Node::insert(&mut f, 388, String::from("Woof"));
    Node::insert(&mut f, 309, String::from("Woof"));
    Node::insert(&mut f, 614, String::from("Woof"));
    Node::insert(&mut f, 549, String::from("Woof"));
    Node::insert(&mut f, 305, String::from("Woof"));
    Node::insert(&mut f, 295, String::from("Woof"));
    Node::insert(&mut f, 532, String::from("Woof"));
    Node::insert(&mut f, 629, String::from("Woof"));
    Node::insert(&mut f, 124, String::from("Woof"));
    Node::insert(&mut f, 307, String::from("Woof"));
    Node::insert(&mut f, 983, String::from("Woof"));
    Node::insert(&mut f, 307, String::from("Woof"));
    Node::insert(&mut f, 768, String::from("Woof"));
    Node::insert(&mut f, 416, String::from("Woof"));
    Node::insert(&mut f, 224, String::from("Woof"));
    Node::insert(&mut f, 10, String::from("Woof"));
    Node::insert(&mut f, 713, String::from("Woof"));
    Node::insert(&mut f, 673, String::from("Woof"));
    Node::insert(&mut f, 84, String::from("Woof"));
    Node::insert(&mut f, 642, String::from("Woof"));
    Node::insert(&mut f, 352, String::from("Woof"));
    Node::insert(&mut f, 644, String::from("Woof"));
    Node::insert(&mut f, 756, String::from("Woof"));
    Node::insert(&mut f, 677, String::from("Woof"));
    Node::insert(&mut f, 977, String::from("Woof"));
    Node::insert(&mut f, 680, String::from("Woof"));
    Node::insert(&mut f, 556, String::from("Woof"));
    Node::insert(&mut f, 821, String::from("Woof"));
    Node::insert(&mut f, 435, String::from("Woof"));
    Node::insert(&mut f, 987, String::from("Woof"));
    Node::insert(&mut f, 67, String::from("Woof"));
    Node::insert(&mut f, 716, String::from("Woof"));
    Node::insert(&mut f, 829, String::from("Woof"));
    Node::insert(&mut f, 786, String::from("Woof"));
    Node::insert(&mut f, 320, String::from("Woof"));
    Node::insert(&mut f, 227, String::from("Woof"));
    Node::insert(&mut f, 953, String::from("Woof"));
    Node::insert(&mut f, 820, String::from("Woof"));
    Node::insert(&mut f, 527, String::from("Woof"));
    Node::insert(&mut f, 315, String::from("Woof"));
    Node::insert(&mut f, 249, String::from("Woof"));
    Node::insert(&mut f, 513, String::from("Woof"));
    Node::insert(&mut f, 683, String::from("Woof"));
    Node::insert(&mut f, 36, String::from("Woof"));
    Node::insert(&mut f, 68, String::from("Woof"));
    Node::insert(&mut f, 252, String::from("Woof"));
    Node::insert(&mut f, 426, String::from("Woof"));
    Node::insert(&mut f, 626, String::from("Woof"));
    Node::insert(&mut f, 608, String::from("Woof"));
    Node::insert(&mut f, 175, String::from("Woof"));
    Node::insert(&mut f, 128, String::from("Woof"));
    Node::insert(&mut f, 573, String::from("Woof"));
    Node::insert(&mut f, 314, String::from("Woof"));
    Node::insert(&mut f, 148, String::from("Woof"));
    Node::insert(&mut f, 527, String::from("Woof"));
    Node::insert(&mut f, 593, String::from("Woof"));
    Node::insert(&mut f, 808, String::from("Woof"));
    Node::insert(&mut f, 870, String::from("Woof"));
    Node::insert(&mut f, 343, String::from("Woof"));
    Node::insert(&mut f, 357, String::from("Woof"));
    Node::insert(&mut f, 577, String::from("Woof"));
    Node::insert(&mut f, 657, String::from("Woof"));
    Node::insert(&mut f, 856, String::from("Woof"));
    Node::insert(&mut f, 368, String::from("Woof"));
    Node::insert(&mut f, 533, String::from("Woof"));
    Node::insert(&mut f, 502, String::from("Woof"));
    Node::insert(&mut f, 531, String::from("Woof"));
    Node::insert(&mut f, 459, String::from("Woof"));
    Node::insert(&mut f, 972, String::from("Woof"));
    Node::insert(&mut f, 130, String::from("Woof"));
    Node::insert(&mut f, 9, String::from("Woof"));
    Node::insert(&mut f, 677, String::from("Woof"));
    Node::insert(&mut f, 821, String::from("Woof"));
    Node::insert(&mut f, 760, String::from("Woof"));
    Node::insert(&mut f, 581, String::from("Woof"));
    Node::insert(&mut f, 97, String::from("Woof"));
    Node::insert(&mut f, 602, String::from("Woof"));
    Node::insert(&mut f, 942, String::from("Woof"));
    Node::insert(&mut f, 653, String::from("Woof"));
    Node::insert(&mut f, 121, String::from("Woof"));
    Node::insert(&mut f, 652, String::from("Woof"));
    Node::insert(&mut f, 897, String::from("Woof"));
    Node::insert(&mut f, 448, String::from("Woof"));
    
    
    
    println!("{:?}", f.lock().unwrap().print_tree());

/*    println!("Key to be discovered?");
    let required_key = read_num();

    match Node::key_position(f.clone(),required_key) {
        Some(x) => {
            println!("Key found");
            println!("{:?}", x);
        }
        None => println!("Key not found"),
    }*/

    println!("Keys to be deleted?");
    let required_key = read_num();
    Node::remove_key(&mut f, required_key);
    println!("{:?}", f.lock().unwrap().print_tree());
}


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
