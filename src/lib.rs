/*Node::insert(&mut f, 1, String::from("Woof"));
Node::insert(&mut f, 2, String::from("Woof"));
Node::insert(&mut f, 3, String::from("Woof"));
Node::insert(&mut f, 4, String::from("Woof"));
Node::insert(&mut f, 5, String::from("Woof"));
Node::insert(&mut f, 6, String::from("Woof"));
Node::insert(&mut f, 7, String::from("Woof"));
Node::insert(&mut f, 8, String::from("Woof"));
Node::insert(&mut f, 9, String::from("Woof"));
Node::insert(&mut f, 10, String::from("Woof"));
Node::insert(&mut f, 11, String::from("Woof"));
Node::insert(&mut f, 12, String::from("Woof"));
Node::insert(&mut f, 13, String::from("Woof"));
Node::insert(&mut f, 14, String::from("Woof"));
Node::insert(&mut f, 15, String::from("Woof"));

    let random_keys = vec![
        42, 763, 198, 571, 925, 314, 689, 147, 832, 456, 259, 673, 918, 34, 507, 742, 189, 621, 954, 276,395, 718, 153, 864, 237, 589, 426, 971, 64, 802,
        345, 678, 913, 52, 729, 184, 537, 860, 293, 641, 478, 815, 126, 369, 702, 945, 211, 584, 837, 162, 497, 730, 85, 412, 759, 204, 631, 978, 351, 694,
        127, 480, 823, 268, 615, 952, 379, 706, 143, 890, 527, 174, 641, 908, 325, 658, 193, 546, 879, 232, 417, 750, 95, 362, 795, 248, 583, 916, 471, 804,
        139, 576, 921, 354, 687, 122, 469, 812, 257, 690, 35, 428, 773, 160, 523, 886, 311, 644, 977, 402, 735, 108, 451, 798, 265, 618, 953, 386, 719, 154,
        171, 504, 857, 292, 625, 988, 453, 786, 119, 552, 895, 330, 663, 996, 429, 762, 195, 548, 911, 374, 737, 100, 463, 816, 251, 604, 947, 380, 713, 166,
        539, 872, 215, 568, 901, 334, 667, 20, 495, 828, 273, 616, 959, 392, 725, 158, 521, 854, 289, 632, 975, 408, 741, 164, 517, 880, 323, 656, 991, 444,
        777, 110, 473, 836, 201, 564, 927, 350, 683, 136, 509, 842, 277, 620, 963, 398, 731, 154, 597, 940];



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


    fn key_position(node: Arc<Mutex<Node>>, key: u32) -> Option<Items> {
        let node_instance = node.lock().unwrap();
        for i in 0..node_instance.input.len() {
            if node_instance.input[i].key == key {
                return Some(node_instance.input[i].clone());
            }
        }

        if key < node_instance.input[0].key {
            return Node::key_position(Arc::clone(&node_instance.children[0]), key);
        } else if key > node_instance.input[node_instance.input.len()-1].key {
            return Node::key_position(Arc::clone(&node_instance.children[node_instance.children.len()-1]), key);
        } else {
            for i in 0..node_instance.input.len() - 1 {
                if key > node_instance.input[i].key && key < node_instance.input[i+1].key {
                    return Node::key_position(Arc::clone(&node_instance.children[i+1]), key);
                }
            }
        }

        None
    }

    
*/



/*Node::insert(&mut f, 602, String::from("Woof"));
Node::insert(&mut f, 607, String::from("Woof"));
Node::insert(&mut f, 492, String::from("Woof"));
Node::insert(&mut f, 477, String::from("Woof"));
Node::insert(&mut f, 366, String::from("Woof"));
Node::insert(&mut f, 439, String::from("Woof"));
Node::insert(&mut f, 492, String::from("Woof"));
Node::insert(&mut f, 648, String::from("Woof"));
Node::insert(&mut f, 717, String::from("Woof"));
Node::insert(&mut f, 311, String::from("Woof"));
Node::insert(&mut f, 896, String::from("Woof"));
Node::insert(&mut f, 394, String::from("Woof"));
Node::insert(&mut f, 19, String::from("Woof"));
Node::insert(&mut f, 929, String::from("Woof"));
Node::insert(&mut f, 725, String::from("Woof"));
Node::insert(&mut f, 497, String::from("Woof"));
Node::insert(&mut f, 553, String::from("Woof"));
Node::insert(&mut f, 167, String::from("Woof"));
Node::insert(&mut f, 162, String::from("Woof"));
Node::insert(&mut f, 651, String::from("Woof"));
Node::insert(&mut f, 396, String::from("Woof"));
Node::insert(&mut f, 580, String::from("Woof"));
Node::insert(&mut f, 444, String::from("Woof"));
Node::insert(&mut f, 613, String::from("Woof"));
Node::insert(&mut f, 924, String::from("Woof"));
Node::insert(&mut f, 927, String::from("Woof"));
Node::insert(&mut f, 268, String::from("Woof"));
Node::insert(&mut f, 431, String::from("Woof"));
Node::insert(&mut f, 84, String::from("Woof"));
Node::insert(&mut f, 487, String::from("Woof"));
Node::insert(&mut f, 320, String::from("Woof"));
Node::insert(&mut f, 676, String::from("Woof"));
Node::insert(&mut f, 685, String::from("Woof"));
Node::insert(&mut f, 17, String::from("Woof"));
Node::insert(&mut f, 258, String::from("Woof"));
Node::insert(&mut f, 361, String::from("Woof"));
Node::insert(&mut f, 783, String::from("Woof"));
Node::insert(&mut f, 842, String::from("Woof"));
Node::insert(&mut f, 425, String::from("Woof"));
Node::insert(&mut f, 582, String::from("Woof"));
Node::insert(&mut f, 832, String::from("Woof"));
Node::insert(&mut f, 163, String::from("Woof"));
Node::insert(&mut f, 842, String::from("Woof"));
Node::insert(&mut f, 572, String::from("Woof"));
Node::insert(&mut f, 464, String::from("Woof"));
Node::insert(&mut f, 561, String::from("Woof"));
Node::insert(&mut f, 391, String::from("Woof"));
Node::insert(&mut f, 316, String::from("Woof"));
Node::insert(&mut f, 17, String::from("Woof"));
Node::insert(&mut f, 719, String::from("Woof"));
Node::insert(&mut f, 892, String::from("Woof"));
Node::insert(&mut f, 607, String::from("Woof"));
Node::insert(&mut f, 127, String::from("Woof"));
Node::insert(&mut f, 768, String::from("Woof"));
Node::insert(&mut f, 552, String::from("Woof"));
Node::insert(&mut f, 420, String::from("Woof"));
Node::insert(&mut f, 264, String::from("Woof"));
Node::insert(&mut f, 545, String::from("Woof"));
Node::insert(&mut f, 130, String::from("Woof"));
Node::insert(&mut f, 287, String::from("Woof"));
Node::insert(&mut f, 991, String::from("Woof"));
Node::insert(&mut f, 665, String::from("Woof"));
Node::insert(&mut f, 623, String::from("Woof"));
Node::insert(&mut f, 318, String::from("Woof"));
Node::insert(&mut f, 201, String::from("Woof"));
Node::insert(&mut f, 809, String::from("Woof"));
Node::insert(&mut f, 783, String::from("Woof"));
Node::insert(&mut f, 135, String::from("Woof"));
Node::insert(&mut f, 972, String::from("Woof"));
Node::insert(&mut f, 378, String::from("Woof"));
Node::insert(&mut f, 651, String::from("Woof"));
Node::insert(&mut f, 517, String::from("Woof"));
Node::insert(&mut f, 153, String::from("Woof"));
Node::insert(&mut f, 894, String::from("Woof"));
Node::insert(&mut f, 252, String::from("Woof"));
Node::insert(&mut f, 505, String::from("Woof"));
Node::insert(&mut f, 637, String::from("Woof"));
Node::insert(&mut f, 814, String::from("Woof"));
Node::insert(&mut f, 386, String::from("Woof"));
Node::insert(&mut f, 375, String::from("Woof"));
Node::insert(&mut f, 685, String::from("Woof"));
Node::insert(&mut f, 98, String::from("Woof"));
Node::insert(&mut f, 561, String::from("Woof"));
Node::insert(&mut f, 204, String::from("Woof"));
Node::insert(&mut f, 892, String::from("Woof"));
Node::insert(&mut f, 580, String::from("Woof"));
Node::insert(&mut f, 819, String::from("Woof"));
Node::insert(&mut f, 832, String::from("Woof"));
Node::insert(&mut f, 38, String::from("Woof"));
Node::insert(&mut f, 344, String::from("Woof"));
Node::insert(&mut f, 817, String::from("Woof"));
Node::insert(&mut f, 866, String::from("Woof"));
Node::insert(&mut f, 588, String::from("Woof"));
Node::insert(&mut f, 83, String::from("Woof"));
Node::insert(&mut f, 968, String::from("Woof"));
Node::insert(&mut f, 445, String::from("Woof"));
Node::insert(&mut f, 979, String::from("Woof"));
Node::insert(&mut f, 642, String::from("Woof"));
Node::insert(&mut f, 727, String::from("Woof"));
Node::insert(&mut f, 914, String::from("Woof"));
*/