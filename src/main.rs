use std::io::{BufRead, BufReader, Read, Write};
extern crate rand;

use std::io;
use std::ptr::read;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use once_cell::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::Rng;
use std::fs::{File, OpenOptions};
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

static NODE_INSTANCE: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

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
    fn new() -> Arc<Mutex<Node>> {
        NODE_INSTANCE.fetch_add(1, Ordering::SeqCst);
        let instance = Arc::new(Mutex::new(Node {
            input: Vec::new(),
            rank: 1,
            children: Vec::new(),
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
                    let k = numbers[0];
                    vec.push(U32OrString::Num(numbers[0]));
                    vec.push(U32OrString::Num(numbers[1]));
                    // println!("{} {}", numbers[0], numbers[1]);
                } else if numbers.len() == 1 {
                    vec.push(U32OrString::Num(numbers[0]));
                    // println!("{}", numbers[0]);
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
        for i in 0..vector_len {
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
        println!("----------------");
        println!("{:?}", node_vec);
        println!("----------------");
        
        let mut required_node = node_vec[0].clone();
        let x = Node::deserialized_data_input(required_node,node_vec);

        println!("KKKKKK {:?}", x);
        Ok(())
    }
    

    fn deserialized_data_input(required_node :DeserializedNode, node_vec: Vec<DeserializedNode>) -> UltraDeserialized {
        let mut node_vec_instance = node_vec;
        let mut x = UltraDeserialized {parent: required_node.clone(), children: Vec::new()};
        if required_node.child_count > 0 {
            let mut i = 0;
            while i < node_vec_instance.len() && required_node.child_count > x.children.len() as u32 {

                if required_node.items[0].rank + 1 == node_vec_instance[i].items[0].rank {
                    x.children.push(UltraDeserialized {parent: node_vec_instance[i].clone(), children: Vec::new()});
                    node_vec_instance.remove(i);
                } else {
                    i += 1;
                }
            }
        }


        if x.parent.child_count > 0 {
            for i in 0..(x.children.len() - 1) {
                let mut z;
                if x.children[i].parent.child_count != 0 {
                    z = Node::deserialized_data_input(x.children[i].parent.clone(),node_vec_instance.clone());
                    x.children.push(z);
                } 
           }
        }
        x
    }
}

fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let mut f = Node::new();
/*    let mut c = 0;
    for i in 0..100 {
        let sec = rand::thread_rng().gen_range(1, 1000);
        Node::insert(&mut f, sec, String::from("Woof"));
        c = c + 1;
        println!("{} - {}", c, sec);
    }*/

    Node::insert(&mut f, 42, String::from("Woof"));
    Node::insert(&mut f, 17, String::from("Woof"));
    Node::insert(&mut f, 89, String::from("Woof"));
    Node::insert(&mut f, 5, String::from("Woof"));
    Node::insert(&mut f, 73, String::from("Woof"));
    Node::insert(&mut f, 31, String::from("Woof"));
    Node::insert(&mut f, 96, String::from("Woof"));
    Node::insert(&mut f, 12, String::from("Woof"));
    Node::insert(&mut f, 58, String::from("Woof"));
    Node::insert(&mut f, 84, String::from("Woof"));
    Node::insert(&mut f, 26, String::from("Woof"));
    Node::insert(&mut f, 63, String::from("Woof"));
    Node::insert(&mut f, 1, String::from("Woof"));
    Node::insert(&mut f, 47, String::from("Woof"));
    Node::insert(&mut f, 100, String::from("Woof"));
    Node::insert(&mut f, 35, String::from("Woof"));
    Node::insert(&mut f, 71, String::from("Woof"));
    Node::insert(&mut f, 19, String::from("Woof"));
    Node::insert(&mut f, 54, String::from("Woof"));
    Node::insert(&mut f, 88, String::from("Woof"));
    Node::insert(&mut f, 7, String::from("Woof"));
    Node::insert(&mut f, 92, String::from("Woof"));

    println!("{:?}", f);
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

/*    for i in 0..100 {
        println!("Keys to be deleted?");
        let required_key = read_num();
        Node::remove_key(&mut f, required_key);
        println!("{:?}", f.lock().unwrap().print_tree());
    }*/
    
/*    let k = Node::all_keys_ordered(&mut f);
    for i in 0..k.len() {
        println!("{} - {}", k[i].key, k[i].value);
    }*/
    
    // Node::serialize(&f).expect("panic message");
    
    
    Node::deserialize().expect("panic message");
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
