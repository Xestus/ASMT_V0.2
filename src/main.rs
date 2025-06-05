use std::io;
use std::sync::{Arc, Mutex};
use once_cell::sync::*;
use lazy_static::lazy_static;


static NODE_SIZE: OnceCell<u8> = OnceCell::new();

#[derive(Debug, Clone, PartialEq)]
struct Items {
    key: u8,
    value: String,
    rank: u8,
}
#[derive(Debug, Clone)]
struct Node {
    input: Vec<Items>,
    rank: u8,
    children: Vec<Arc<Mutex<Node>>>,
}

lazy_static! {
    static ref NODE_INSTANCE: Mutex<Vec<Arc<Mutex<Node>>>> = Mutex::new(Vec::new());
}

impl Node {
    fn new() -> Arc<Mutex<Node>> {
        let instance = Arc::new(Mutex::new(Node {
            input: Vec::new(),
            rank: nodes_count(),
            children: Vec::new(),
        }));
        NODE_INSTANCE.lock().unwrap().push(instance.clone());
        instance
    }
    fn insert(&mut self, k: u8, v: String) -> () {
        let meow = self.input.len();
        if meow > 1 && !self.children.is_empty() {
            self.input.push(Items {key: k, value: v.clone(), rank: self.rank});
            println!("Splitted");
            self.more_than_one_parent_key_when_children_not_empty();
            // Create a separate implementation for it.
        }
        else if !self.children.is_empty() {
            self.input.push(Items {key: k, value: v, rank: self.rank});
            self.children_not_empty();
        } else {
            self.input.push(Items {key: k, value: v, rank: self.rank});
            if NODE_SIZE.get().unwrap().clone() < self.input.len() as u8 {
                self.max_size_exceeded();
            }

            bubble_sort(&mut self.input);
        }
    }

    fn more_than_one_parent_key_when_children_not_empty(&mut self) {
        println!("AAAAA --------------------------------------------------------------------------------");
        println!("MEOMOTW {:?}", self);
        println!("AAAAA --------------------------------------------------------------------------------");

        let mut placeholder_struct = self.clone();

        let size_of_main_node = self.input.len();

        self.input.clear();
        for i in 0..size_of_main_node - 1 {
            self.input.push(placeholder_struct.input[i].clone());
            self.sort_main_nodes();
        }
        println!("Stupid {:?}", self.input);
        self.sort_children_nodes();


        for i in 1..size_of_main_node {
            placeholder_struct.input[i].rank = self.input[0].rank + 1;
            if placeholder_struct.input[i].key != self.input[0].key {
                self.children.push(Arc::new(Mutex::new(
                    Node {
                        input : vec!(placeholder_struct.input[i].clone()),
                        rank: self.rank + 1,
                        children: Vec::new(),
                    }))
                )

            }
        }
        let k = self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>();
        for i in k {
            if i.lock().unwrap().input.len() < (NODE_SIZE.get().unwrap().clone()/2) as usize {
                println!("K");
                self.min_size_subceeded();
            }
        }

        self.sort_children_nodes();
    }
    
    fn max_size_exceeded(&mut self)  {
        bubble_sort(&mut self.input);

        let struct_one = Node::new();
        let struct_two = Node::new();
        
        let items_size = self.input.len();
        let breaking_point = (items_size + 1)/2;
        let temp_storage = self.clone();
        let mut count = 0;
        let node_size = NODE_SIZE.get().unwrap().clone() as usize;

        self.input.clear();
         for _v in temp_storage.input.iter() {
            count +=1;

            if count == breaking_point {
                self.input.push(temp_storage.input[count-1].clone());
            } else if count > breaking_point {
                struct_two.lock().unwrap().input.push(temp_storage.input[count- 1].clone());
                struct_two.lock().unwrap().input[count - node_size].rank = temp_storage.rank + 1;
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone());
                struct_one.lock().unwrap().input[count - 1].rank = temp_storage.rank + 1;
            }
        }
        
        self.children.push(struct_one.clone());
        self.children.push(struct_two.clone());
        
        for k in self.children.iter() {
            k.lock().unwrap().rank = self.rank + 1;
        }

    }
    
    fn min_size_subceeded(&mut self) {

        println!("SSSSSSS --------------------------------------------------------------------------------");
        println!("SSSSSSSSSS {:?}", self);
        println!("SSSSSSSS --------------------------------------------------------------------------------");

        let mut count = 0;
        // println!("K {}", self.input.len());
/*        if self.input.len() > 1 {
            for i in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
                count += 1;
                let k = i.lock().unwrap().input.clone();
                if k.len() < (NODE_SIZE.get().unwrap().clone()/2) as usize {
                    println!("OOOOOOOOOOOOOO");
                }
            }
        } else*/ if self.children.len() > 1 {
            for i in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
                count += 1;
                let k = i.lock().unwrap().input.clone();
                if k.len() < (NODE_SIZE.get().unwrap().clone()/2) as usize {

                    self.min_size_1(k,count);
                }
            }
        }

/*        println!("BBBBB --------------------------------------------------------------------------------");
        println!("MEOMOTW {:?}", self);
        println!("ZZZZZZZ --------------------------------------------------------------------------------");
*/
    }

    fn min_size_1(&mut self,k: Vec<Items>, count: usize) {

        println!("PP {:?}", k);
        let mut temp_vector = Vec::new();
        
        if self.input.len() < 3 {
            if k[0].key < self.input[0].key {
                println!("{:?}", k[0].key);
                self.children[0].lock().unwrap().input.push(k[0].clone());
                self.children.remove(count-1);
            } else if k[0].key > self.input[self.input.len()-1].key {
                println!("{:?}", self);
                self.children[self.input.len()].lock().unwrap().input.push(k[0].clone());
                // println!("ZZZZ {:?}", self);
                self.children.remove(count-1);
            } else if k[0].key == self.input[0].key || k[0].key == self.input[self.input.len()-1].key {
                for i in self.input.iter().cloned().collect::<Vec<Items>>() {
                    // self.min_size_2(k.clone(), count);
                    println!("{:?}", i);
                    println!("Minamoto");
                    temp_vector.push(i.clone());
                }
            } else {
                // This shit does absolutely nothing.
                /*            for i in 1..self.input.len()-1 {
                                println!("{}", self.input[i].key);
                                if k[0].key != self.input[i].key {
                                    println!("///////////////////////////////////////////////////////////");
                                    println!("{:?}", k[0].key);
                                    println!("MEow");
                                    println!("///////////////////////////////////////////////////////////");
                
                                }
                            }
                */        }
        }

        
        
        for p in temp_vector.iter() {
            let mut c = 0;
            for z in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
                c = c +1;
                for v in z.lock().unwrap().input.iter() {
                    if p.key == v.key {
                        self.children.remove( c - 1);
                    }
                }
            }
        }

        println!("ZZ {:?}", self.children);

        self.sort_children_items();
        if self.children[self.input.len()].lock().unwrap().input.len() > NODE_SIZE.get().unwrap().clone() as usize {
            self.children[self.input.len()].lock().unwrap().max_children_size_exceeded();
        } else if self.children[0].lock().unwrap().input.len() > NODE_SIZE.get().unwrap().clone() as usize {
            self.children[0].lock().unwrap().max_children_size_exceeded();
        }

        // Add a function that takes the last value (if single) and merges it to required function. 
        
        // HACk: A random rank 1 appears inside the code, so, picked 3 keys from children for comparison for selected number
        let picked_struct = self.children[self.children.len() - 1].lock().unwrap().clone();
        let random_child = self.children[0].lock().unwrap().clone().rank;
        if picked_struct.input.len() < (NODE_SIZE.get().unwrap().clone()/2) as usize && picked_struct.rank == random_child {
            println!("{:?}", self.children);
            self.min_size_2();
        }
        self.children_iteration();
    }
    
    fn min_size_2(&mut self) {
        let x = self.children[self.children.len() - 1].lock().unwrap().input.clone();
        self.children.remove(self.children.len() - 1);
        
        if x[0].key < self.input[0].key {
            self.children[0].lock().unwrap().input.push(x[0].clone());
        } else if x[0].key > self.input[self.input.len()-1].key {
            self.children[self.input.len()-1].lock().unwrap().input.push(x[0].clone());
        } else {
            for i in 0..self.input.len() - 1 {
                if x[0].key > self.input[i].key && x[0].key < self.input[i+1].key {
                    self.children[i+1].lock().unwrap().input.push(x[0].clone());
                }
            }
        }
        
    }

    fn children_iteration(&mut self) -> () {
        let mut c = 0;
        for i in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
            c = c + 1;
            if i.lock().unwrap().rank == self.rank {
                self.input.push(i.lock().unwrap().input[0].clone());
                for k in i.lock().unwrap().children.iter() {
                    self.children.push(k.clone());
                }
                i.lock().unwrap().children.clear();

                self.children.remove(c-1);
            }
        }
    }
    
    fn max_children_size_exceeded(&mut self) {
        bubble_sort(&mut self.input);

        let struct_one = Node::new();
        let struct_two = Node::new();
        
        // HACK: struct created below has the rank + 1 of the upper one, only in this method. Temporary Fix:
        struct_two.lock().unwrap().rank = struct_one.lock().unwrap().rank;
        
        let items_size = self.input.len();
        let breaking_point = (items_size + 1)/2;
        let temp_storage = self.clone();
        let mut count = 0;
        self.input.clear();
        for _v in temp_storage.input.iter() {
            count +=1;

            if count == breaking_point {
                self.input.push(temp_storage.input[count-1].clone());
                self.input[0].rank = temp_storage.rank - 1;
            } else if count > breaking_point {
                struct_two.lock().unwrap().input.push(temp_storage.input[count - 1].clone());
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone());
            }
        }

        self.rank = self.rank -1;
        struct_one.lock().unwrap().rank = self.rank + 1;
        struct_two.lock().unwrap().rank = self.rank + 1;
        self.children.push(struct_one.clone());
        self.children.push(struct_two.clone());
        
    }

    // I hate my life. What does it even do?
    fn children_nodes_overflow(&mut self) {
        
    }
    
    fn children_not_empty(&mut self) -> () {
        let mut placeholder_struct = self.clone();

        let size_of_main_node = self.input.len();

        // println!("ZZZZZ {:?}", self.input);
        self.input.clear();
        self.input.push(placeholder_struct.input[0].clone());
        // println!("{:?}", self.input);
        self.sort_children_nodes();


        for i in 1..size_of_main_node {
            placeholder_struct.input[i].rank = self.input[0].rank + 1;
            if placeholder_struct.input[i].key != self.input[0].key {
                self.children.push(Arc::new(Mutex::new(
                    Node {
                        input : vec!(placeholder_struct.input[i].clone()),
                        rank: self.rank + 1,
                        children: Vec::new(),
                    }))
                )

            } else {
                println!("The entered key already exists.");
            }
        }
            let k = self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>();

        for i in k {
            if i.lock().unwrap().input.len() < (NODE_SIZE.get().unwrap().clone()/2) as usize {
                self.min_size_subceeded();
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
fn nodes_count() -> u8 {
    let k = NODE_INSTANCE.lock().unwrap().clone();
    let mut temp_vec:Vec<u8> = Vec::new();
    for i in k.iter() {
        if let Ok(i) = i.try_lock() {
            temp_vec.push(i.rank);
        } else {
            println!("Could not lock the mutex");
        }
    }
    temp_vec.sort();
    temp_vec.reverse();
    if temp_vec.len() == 0 {
        1
    } else {
        temp_vec[0] + 1
    }
}

fn bubble_sort(arr: &mut [Items]) {
    let n = arr.len();
    for i in 0..n {
        for j in 0..n - 1 - i {
            if arr[j].key > arr[j + 1].key {
                arr.swap(j, j + 1);
            }
        }
    }
}


fn main() {

    NODE_SIZE.set(4).expect("Failed to set size");
    let f = Node::new();
    f.lock().unwrap().insert(10, String::from("a"));
    f.lock().unwrap().insert(40, String::from("Squeak"));
    f.lock().unwrap().insert(20, String::from("Woof"));
    f.lock().unwrap().insert(45, String::from("Meow"));
    f.lock().unwrap().insert(30, String::from("Caw-Caw"));
    f.lock().unwrap().insert(13, String::from("Chirp"));
    f.lock().unwrap().insert(43, String::from("Ribbit"));
    f.lock().unwrap().insert(42, String::from("Purr"));
    f.lock().unwrap().insert(18, String::from("Neigh"));
    f.lock().unwrap().insert(7, String::from("Myahhh"));
    f.lock().unwrap().insert(50, String::from("Oink-Oink"));
    f.lock().unwrap().insert(32, String::from("Bah"));
    f.lock().unwrap().insert(12, String::from("Mahh"));



    //ToDO: Combine the newly added keys to required child vector
    println!("--------------------------------------------------------------------------------");
    println!("A  {:?}", f.try_lock().unwrap().input);
    println!("--------------------------------------------------------------------------------");
    println!("B  {:?}", f.try_lock().unwrap().children);
    println!("--------------------------------------------------------------------------------");
    println!("A  {:?}", f);

}


fn read_num() -> u8 {
    let mut inp = String::new();
    io::stdin().read_line(&mut inp).expect("Failed to read line");
    let n: u8 = inp.trim().parse().expect("Not a number");
    n
}

fn read_string() -> String {
    let mut inp = String::new();
    io::stdin().read_line(&mut inp).expect("Failed to read line");
    inp.trim().to_string()
}
