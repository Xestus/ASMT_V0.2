extern crate rand;

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
        z = Node::tree_split_check(z);
        z = Node::min_size_check(z);

        z = Node::overflow_check(z);
        z = Node::min_size_check(z);
        z.sort_main_nodes();
        z = Node::tree_split_check(z);
        z = Node::rank_correction(z);
        z = Node::sort_everything(z);
        
        z = Node::overflow_check(z);

        z = Node::min_size_check(z);
        z = Node::tree_split_check(z);
        z = Node::rank_correction(z);

        // z = Node::overflow_check(z);
        // z = Node::min_size_check(z);
        // z = Node::tree_split_check(z);
        // z = Node::rank_correction(z);
        // z = Node::sort_everything(z);

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

/*        println!("------------------------");
        println!("AAA {:?}", self_node.print_tree());
        println!("------------------------");
*/
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

        println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        println!("{:?}", self_instance.print_tree());
        println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");

        let mut required_child = vec![self_instance.children[self_instance.children.len()-2].lock().unwrap().clone()];
        required_child.push(self_instance.children[self_instance.children.len()-1].lock().unwrap().clone());

        let x = self_instance.children[self_instance.children.len()-2].lock().unwrap().input.len();
        let y = self_instance.children[self_instance.children.len()-1].lock().unwrap().input.len();

        // HACK: only selected the children with no children of their own to be added under new node.
        // TODO: Hank no work as most of the time, you need to merge childrens with childrens.

        let mut for_x = Vec::new();
        let mut for_y = Vec::new();

        for i in 0..2 {
            println!("{}", i);
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

        println!("{:?} {:?}", for_x, for_y);
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
            println!("{:?}", self_instance.print_tree());
        }

/*        for _i in 0..x+1 {
            let mut p = 0;

            while !self_instance.children[p].lock().unwrap().children.is_empty() {
                p = p + 1;

                if self_instance.children.len() -2 == p {
                    p = 0;
                    break;
                }
            }

            let temp = self_instance.children[p].lock().unwrap().clone();
            self_instance.children[self_instance.children.len()-2].lock().unwrap().children.push(Arc::new(Mutex::new(temp)));
            self_instance.children.remove(p);
        }

        for _i in 0..y+1 {
            let mut p = 0;
            while !self_instance.children[p].lock().unwrap().children.is_empty() {
                p = p + 1;
                if self_instance.children.len() -2 == p {
                    p = 0;
                    break;
                }
            }
            self_instance.children[self_instance.children.len()-1].lock().unwrap().children.push(self_instance.children[p].clone());
            self_instance.children.remove(p);
        }*/

        self_instance
    }
    fn rank_correction(self_node: MutexGuard<Node>) -> MutexGuard<Node> {
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
/*        self_instance.children[0].lock().unwrap().rank = self_instance.children[self_instance.children.len()-1].lock().unwrap().rank + 1;
        let k = self_instance.children[0].lock().unwrap().input.len();
        for j in 0..k {
            self_instance.children[0].lock().unwrap().input[j].rank = self_instance.children[self_instance.children.len()-1].lock().unwrap().rank + 1;
        }*/
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
        println!("------------------------");
        println!("AAA {:?}", self_node.print_tree());
        println!("------------------------");

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

        println!("##################################################");
        println!("AAA {:?}", x.print_tree());
        println!("######################################################3");

        x
    }
    
    fn sort_children_nodes(&mut self) {
        self.children.sort_by(|a, b| {a.lock().unwrap().input[0].key.cmp(&b.lock().unwrap().input[0].key)});
    }
    fn sort_main_nodes(&mut self) {
        self.input.sort_by(|a, b| {a.key.cmp(&b.key)});
    }
    fn sort_everything(node: MutexGuard<Node>) -> MutexGuard<Node> {
        let mut current_node = node;
        current_node.sort_main_nodes();

        current_node.sort_children_nodes();

        let children = current_node.clone().children;
        
        for i in 0..children.len() {
            Node::sort_everything(children[i].lock().unwrap());
        }
        
        current_node
    }

}

fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let mut f = Node::new();
    /*for i in 0..200 {
        let sec = rand::thread_rng().gen_range(1, 1000);
        Node::insert(&mut f, sec, String::from("Woof"));
    }*/

    Node::insert(&mut f, 1243, String::from("Woof"));
    Node::insert(&mut f, 872, String::from("Woof"));
    Node::insert(&mut f, 1495, String::from("Woof"));
    Node::insert(&mut f, 356, String::from("Woof"));
    Node::insert(&mut f, 1128, String::from("Woof"));
    Node::insert(&mut f, 765, String::from("Woof"));
    Node::insert(&mut f, 431, String::from("Woof"));
    Node::insert(&mut f, 987, String::from("Woof"));
    Node::insert(&mut f, 532, String::from("Woof"));
    Node::insert(&mut f, 1199, String::from("Woof"));
    Node::insert(&mut f, 204, String::from("Woof"));
    Node::insert(&mut f, 678, String::from("Woof"));
    Node::insert(&mut f, 1456, String::from("Woof"));
    Node::insert(&mut f, 321, String::from("Woof"));
    Node::insert(&mut f, 908, String::from("Woof"));
    Node::insert(&mut f, 115, String::from("Woof"));
    Node::insert(&mut f, 1378, String::from("Woof"));
    Node::insert(&mut f, 599, String::from("Woof"));
    Node::insert(&mut f, 1042, String::from("Woof"));
    Node::insert(&mut f, 777, String::from("Woof"));
    Node::insert(&mut f, 234, String::from("Woof"));
    Node::insert(&mut f, 1299, String::from("Woof"));
    Node::insert(&mut f, 456, String::from("Woof"));
    Node::insert(&mut f, 1087, String::from("Woof"));
    Node::insert(&mut f, 643, String::from("Woof"));
    Node::insert(&mut f, 1421, String::from("Woof"));
    Node::insert(&mut f, 88, String::from("Woof"));
    Node::insert(&mut f, 1210, String::from("Woof"));
    Node::insert(&mut f, 377, String::from("Woof"));
    Node::insert(&mut f, 955, String::from("Woof"));
    Node::insert(&mut f, 512, String::from("Woof"));
    Node::insert(&mut f, 1344, String::from("Woof"));
    Node::insert(&mut f, 199, String::from("Woof"));
    Node::insert(&mut f, 826, String::from("Woof"));
    Node::insert(&mut f, 1167, String::from("Woof"));
    Node::insert(&mut f, 478, String::from("Woof"));
    Node::insert(&mut f, 1023, String::from("Woof"));
    Node::insert(&mut f, 711, String::from("Woof"));
    Node::insert(&mut f, 1482, String::from("Woof"));
    Node::insert(&mut f, 155, String::from("Woof"));
    Node::insert(&mut f, 1265, String::from("Woof"));
    Node::insert(&mut f, 394, String::from("Woof"));
    Node::insert(&mut f, 867, String::from("Woof"));
    Node::insert(&mut f, 1134, String::from("Woof"));
    Node::insert(&mut f, 522, String::from("Woof"));
    Node::insert(&mut f, 1399, String::from("Woof"));
    Node::insert(&mut f, 277, String::from("Woof"));
    Node::insert(&mut f, 944, String::from("Woof"));
    Node::insert(&mut f, 611, String::from("Woof"));
    Node::insert(&mut f, 1288, String::from("Woof"));
    Node::insert(&mut f, 433, String::from("Woof"));
    Node::insert(&mut f, 1001, String::from("Woof"));
    Node::insert(&mut f, 788, String::from("Woof"));
    Node::insert(&mut f, 1467, String::from("Woof"));
    Node::insert(&mut f, 122, String::from("Woof"));
    Node::insert(&mut f, 1333, String::from("Woof"));
    Node::insert(&mut f, 499, String::from("Woof"));
    Node::insert(&mut f, 1056, String::from("Woof"));
    Node::insert(&mut f, 822, String::from("Woof"));
    Node::insert(&mut f, 1177, String::from("Woof"));
    Node::insert(&mut f, 344, String::from("Woof"));
    Node::insert(&mut f, 911, String::from("Woof"));
    Node::insert(&mut f, 588, String::from("Woof"));
    Node::insert(&mut f, 1429, String::from("Woof"));
    Node::insert(&mut f, 177, String::from("Woof"));
    Node::insert(&mut f, 1244, String::from("Woof"));
    Node::insert(&mut f, 411, String::from("Woof"));
    Node::insert(&mut f, 966, String::from("Woof"));
    Node::insert(&mut f, 133, String::from("Woof"));
    Node::insert(&mut f, 1355, String::from("Woof"));
    Node::insert(&mut f, 622, String::from("Woof"));
    Node::insert(&mut f, 1099, String::from("Woof"));
    Node::insert(&mut f, 755, String::from("Woof"));
    Node::insert(&mut f, 1488, String::from("Woof"));
    Node::insert(&mut f, 266, String::from("Woof"));
    Node::insert(&mut f, 833, String::from("Woof"));
    Node::insert(&mut f, 1144, String::from("Woof"));
    Node::insert(&mut f, 477, String::from("Woof"));
    Node::insert(&mut f, 1022, String::from("Woof"));
    Node::insert(&mut f, 699, String::from("Woof"));
    Node::insert(&mut f, 1477, String::from("Woof"));
    Node::insert(&mut f, 144, String::from("Woof"));
    Node::insert(&mut f, 1276, String::from("Woof"));
    Node::insert(&mut f, 355, String::from("Woof"));
    Node::insert(&mut f, 922, String::from("Woof"));
    Node::insert(&mut f, 511, String::from("Woof"));
    Node::insert(&mut f, 1388, String::from("Woof"));
    Node::insert(&mut f, 233, String::from("Woof"));
    Node::insert(&mut f, 999, String::from("Woof"));
    Node::insert(&mut f, 666, String::from("Woof"));
    Node::insert(&mut f, 1111, String::from("Woof"));
    Node::insert(&mut f, 444, String::from("Woof"));
    Node::insert(&mut f, 888, String::from("Woof"));
    Node::insert(&mut f, 1222, String::from("Woof"));
    Node::insert(&mut f, 333, String::from("Woof"));
    Node::insert(&mut f, 777, String::from("Woof"));
    Node::insert(&mut f, 111, String::from("Woof"));
    Node::insert(&mut f, 555, String::from("Woof"));
    Node::insert(&mut f, 999, String::from("Woof"));
    Node::insert(&mut f, 222, String::from("Woof"));
    Node::insert(&mut f, 666, String::from("Woof"));
    Node::insert(&mut f, 1111, String::from("Woof"));
    Node::insert(&mut f, 444, String::from("Woof"));
    Node::insert(&mut f, 888, String::from("Woof"));
    Node::insert(&mut f, 1234, String::from("Woof"));
    Node::insert(&mut f, 567, String::from("Woof"));
    Node::insert(&mut f, 890, String::from("Woof"));
    Node::insert(&mut f, 432, String::from("Woof"));
    Node::insert(&mut f, 1098, String::from("Woof"));
    Node::insert(&mut f, 765, String::from("Woof"));
    Node::insert(&mut f, 321, String::from("Woof"));
    Node::insert(&mut f, 987, String::from("Woof"));
    Node::insert(&mut f, 654, String::from("Woof"));
    Node::insert(&mut f, 210, String::from("Woof"));
    Node::insert(&mut f, 543, String::from("Woof"));
    Node::insert(&mut f, 876, String::from("Woof"));
    Node::insert(&mut f, 1209, String::from("Woof"));
    Node::insert(&mut f, 345, String::from("Woof"));
    Node::insert(&mut f, 678, String::from("Woof"));
    Node::insert(&mut f, 912, String::from("Woof"));
    Node::insert(&mut f, 234, String::from("Woof"));
    Node::insert(&mut f, 567, String::from("Woof"));
    Node::insert(&mut f, 890, String::from("Woof"));
    Node::insert(&mut f, 123, String::from("Woof"));
    Node::insert(&mut f, 456, String::from("Woof"));
    Node::insert(&mut f, 789, String::from("Woof"));
    Node::insert(&mut f, 1023, String::from("Woof"));
    Node::insert(&mut f, 135, String::from("Woof"));
    Node::insert(&mut f, 468, String::from("Woof"));
    Node::insert(&mut f, 791, String::from("Woof"));
    Node::insert(&mut f, 1124, String::from("Woof"));
    Node::insert(&mut f, 357, String::from("Woof"));
    Node::insert(&mut f, 680, String::from("Woof"));
    Node::insert(&mut f, 913, String::from("Woof"));
    Node::insert(&mut f, 246, String::from("Woof"));
    Node::insert(&mut f, 579, String::from("Woof"));
    Node::insert(&mut f, 802, String::from("Woof"));
    Node::insert(&mut f, 1135, String::from("Woof"));
    Node::insert(&mut f, 368, String::from("Woof"));
    Node::insert(&mut f, 691, String::from("Woof"));
    Node::insert(&mut f, 924, String::from("Woof"));
    Node::insert(&mut f, 257, String::from("Woof"));
    Node::insert(&mut f, 580, String::from("Woof"));
    Node::insert(&mut f, 803, String::from("Woof"));

    Node::insert(&mut f, 1136, String::from("Woof"));

    // Node::insert(&mut f, 369, String::from("Woof"));
    // Node::insert(&mut f, 692, String::from("Woof"));
    // Node::insert(&mut f, 925, String::from("Woof"));
    // Node::insert(&mut f, 258, String::from("Woof"));
    // Node::insert(&mut f, 581, String::from("Woof"));
    // Node::insert(&mut f, 804, String::from("Woof"));
    // Node::insert(&mut f, 1137, String::from("Woof"));
    // Node::insert(&mut f, 370, String::from("Woof"));
    // Node::insert(&mut f, 693, String::from("Woof"));
    // Node::insert(&mut f, 926, String::from("Woof"));
    // Node::insert(&mut f, 259, String::from("Woof"));
    // Node::insert(&mut f, 582, String::from("Woof"));
    // Node::insert(&mut f, 805, String::from("Woof"));
    // Node::insert(&mut f, 1138, String::from("Woof"));
    // Node::insert(&mut f, 371, String::from("Woof"));
    // Node::insert(&mut f, 694, String::from("Woof"));
    // Node::insert(&mut f, 927, String::from("Woof"));
    // Node::insert(&mut f, 260, String::from("Woof"));
    // Node::insert(&mut f, 583, String::from("Woof"));
    // Node::insert(&mut f, 806, String::from("Woof"));
    // Node::insert(&mut f, 1139, String::from("Woof"));
    // Node::insert(&mut f, 372, String::from("Woof"));
    // Node::insert(&mut f, 695, String::from("Woof"));
    // Node::insert(&mut f, 928, String::from("Woof"));
    
    
    // Node::insert(&mut f, 261, String::from("Woof"));
    // Node::insert(&mut f, 584, String::from("Woof"));
    // Node::insert(&mut f, 807, String::from("Woof"));
    // Node::insert(&mut f, 1140, String::from("Woof"));
    // Node::insert(&mut f, 373, String::from("Woof"));
    // Node::insert(&mut f, 696, String::from("Woof"));
    // Node::insert(&mut f, 929, String::from("Woof"));
    // Node::insert(&mut f, 262, String::from("Woof"));
    // Node::insert(&mut f, 585, String::from("Woof"));
    // Node::insert(&mut f, 808, String::from("Woof"));
    // Node::insert(&mut f, 1141, String::from("Woof"));
    // Node::insert(&mut f, 374, String::from("Woof"));
    // Node::insert(&mut f, 697, String::from("Woof"));
    // Node::insert(&mut f, 930, String::from("Woof"));
    // Node::insert(&mut f, 263, String::from("Woof"));
    // Node::insert(&mut f, 586, String::from("Woof"));
    // Node::insert(&mut f, 809, String::from("Woof"));
    // Node::insert(&mut f, 1142, String::from("Woof"));
    // Node::insert(&mut f, 375, String::from("Woof"));
    // Node::insert(&mut f, 698, String::from("Woof"));
    // Node::insert(&mut f, 931, String::from("Woof"));
    // Node::insert(&mut f, 264, String::from("Woof"));
    // Node::insert(&mut f, 587, String::from("Woof"));
    // Node::insert(&mut f, 810, String::from("Woof"));
    // Node::insert(&mut f, 1143, String::from("Woof"));


    


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
