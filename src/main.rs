use std::io;
use std::sync::{Arc, Mutex};
use once_cell::sync::*;
use lazy_static::lazy_static;


static NODE_SIZE: OnceCell<u8> = OnceCell::new();

#[derive(Debug, Clone)]
struct Items {
    key: u8,
    value: String,
    rank: u8,
}
#[derive(Debug, Clone)]
struct Node {
    input: Vec<Items>,
    max_size: u8,
    min_size: u8,
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
            max_size: NODE_SIZE.get().unwrap().clone(),
            min_size: NODE_SIZE.get().unwrap().clone() /2,
            rank: nodes_count(), // modify nodes_count() to give accurate rank number
            children: Vec::new(),
        }));
        NODE_INSTANCE.lock().unwrap().push(instance.clone());
        instance
    }

    fn insert(&mut self, k: u8, v: String) -> () {
        self.input.push(Items {key: k, value: v, rank: self.rank});
        if !self.children.is_empty() {
            self.children_not_empty();
        } else {
            if self.max_size < self.input.len() as u8 {
                self.max_size_exceeded();
            }

            bubble_sort(&mut self.input);
        }
    }
    
    fn max_size_exceeded(&mut self)  {
        // let newNode = vec
        // tried to create a new node for else if data but rank issue. Wishing to implement a function that checks whether current rank is occupied or not.
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
                println!("{}   {}     {}    {}", count, node_size, breaking_point, items_size);
                struct_two.lock().unwrap().input.push(temp_storage.input[count- 1].clone());
                struct_two.lock().unwrap().input[count - node_size].rank = temp_storage.rank + 1;
            } else if count < breaking_point {
                struct_one.lock().unwrap().input.push(temp_storage.input[count-1].clone());
                struct_one.lock().unwrap().input[count - 1].rank = temp_storage.rank + 1;
            }
        }
        
        self.children.push(struct_two.clone());
        self.children.push(struct_one.clone());
        
        for k in self.children.iter() {
            k.lock().unwrap().rank = self.rank + 1;
        }
    }
    
    fn min_size_subceeded(&mut self) {
        let mut count = 0;
        if self.children.len() > 1 {
            for i in self.children.iter().cloned().collect::<Vec<Arc<Mutex<Node>>>>() {
                count += 1;
                let k = i.lock().unwrap().input.clone();
                if k.len() < self.min_size as usize {
                    self.min_size_1(k,count);
                }
            }
        }
    }

    fn min_size_1(&mut self,k: Vec<Items>, count: usize) {
        if k[0].key < self.input[0].key {
            self.children[0].lock().unwrap().input.push(k[0].clone());
            self.children.remove(count-1);
        } else if k[0].key > self.input[self.input.len()-1].key {
            self.children[self.input.len()].lock().unwrap().input.push(k[0].clone());
            self.children.remove(count-1);
        } else {
            for i in self.input.iter().cloned().collect::<Vec<Items>>() {
                println!("{}", i.key);
                self.min_size_2(k.clone());
            }
        }
        self.sort_children_items();
        println!("{} {}", self.children[self.input.len()].lock().unwrap().input.len(), NODE_SIZE.get().unwrap().clone());
        if self.children[self.input.len()].lock().unwrap().input.len() > NODE_SIZE.get().unwrap().clone() as usize {
            self.children[self.input.len()].lock().unwrap().max_size_exceeded();
        }
    }
    // todo: Modify the given fn to serve values that aren't lowest or highest root keys.
    fn min_size_2(&mut self,k: Vec<Items>) {
        
    }
    
    // I hate my life.
    fn children_nodes_overflow(&mut self) {
        
    }
    
    fn children_not_empty(&mut self) -> () {
        let mut placeholder_struct = self.clone();
        
        let size_of_main_node = self.input.len();

/*        println!("#######################################################################################");
        println!("{:?}", self.children);
        println!("#######################################################################################");
*/
        self.input.clear();
        self.input.push(placeholder_struct.input[0].clone());
        println!("{:?}", self.children );
        self.sort_children_nodes();
        println!("{:?}", self.children );


        for i in 1..size_of_main_node {
             // to_be_assigned.lock().unwrap().input.push(placeholder_struct.input[i].clone());
            println!("{}", placeholder_struct.input[i].key);
            println!("{}", self.input[0].key);
            placeholder_struct.input[i].rank = self.input[0].rank + 1;
            if placeholder_struct.input[i].key < self.input[0].key {
                // ToDo: Combine the key into respective node. DONE
                
            } else if placeholder_struct.input[i].key > self.input[0].key {
                // self.children.push(Arc::new(Mutex::new(placeholder_struct.clone())))
                self.children.push(Arc::new(Mutex::new(
                    Node {
                         input : vec!(placeholder_struct.input[i].clone()),
                        max_size: NODE_SIZE.get().unwrap().clone(),
                        min_size: NODE_SIZE.get().unwrap().clone() /2,
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
            if i.lock().unwrap().input.len() < self.min_size as usize {
                self.min_size_subceeded();
            }
        }
        
        self.sort_children_nodes();

/*        println!("--------------------------------------------------------------------------------");
        println!("{:?}", self.children);
        println!("--------------------------------------------------------------------------------");
*/

/*                println!("--------------------------------------------------------------------------------");
                println!("{:?} \n {:?} \n {}", self.input,self.children, self.rank);
                println!("--------------------------------------------------------------------------------");
                println!("--------------------------------------------------------------------------------");
*/
    }
    
    fn sort_children_nodes(&mut self) {
        self.children.sort_by(|a, b| {a.lock().unwrap().input[0].key.cmp(&b.lock().unwrap().input[0].key)});
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
    f.lock().unwrap().insert(1, String::from("a"));
    f.lock().unwrap().insert(2, String::from("Squeak"));
    f.lock().unwrap().insert(5, String::from("Woof"));
    f.lock().unwrap().insert(3, String::from("Meow"));
    f.lock().unwrap().insert(6, String::from("Caw-Caw"));

    
    //ToDO: Combine the newly added keys to required child vector

    f.lock().unwrap().insert(4, String::from("Chirp"));
    f.lock().unwrap().insert(7, String::from("Myahhh"));
    f.lock().unwrap().insert(8, String::from("Oink-Oink"));
    println!("--------------------------------------------------------------------------------");
    println!("A  {:?}", f.try_lock().unwrap().input);
    println!("--------------------------------------------------------------------------------");
    println!("B  {:?}", f.try_lock().unwrap().children);
    println!("--------------------------------------------------------------------------------");
    
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

/*#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {

    }
}
*/

/*struct Poopa {
    apple: Vec<u32>,
}
impl Poopa {
    fn new() -> Arc<Mutex<Poopa>> {
        
        Arc::new(Mutex::new(Poopa { apple: Vec::new() }))
    }
    
    fn stuffs() -> () {
        Poopa::new();
        println!("Apple");
    }
}


*/