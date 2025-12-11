use crate::btree::node::Node;
use crate::MVCC::versions::Version;

impl Node {
    /// Pretty print the entire tree with all versions
    pub fn print_tree(&self) {
        println!("B-Tree Structure (All Versions)");
        println!("{}", "=".repeat(50));
        self.print_tree_recursive("", true, 0);
        println!();
    }

    fn print_tree_recursive(&self, prefix: &str, is_last: bool, depth: usize) {
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

        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        for (i, child_arc) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;

            match child_arc.read() {
                Ok(child) => {
                    child.print_tree_recursive(&child_prefix, is_last_child, depth + 1);
                }
                Err(_) => {
                    println!("{}{}[POISONED RWLOCK]",
                             child_prefix,
                             if is_last_child { "└── " } else { "├── " });
                }
            }
        }
    }

    fn format_items(&self) -> String {
        if self.input.is_empty() {
            return "empty".to_string();
        }

        let items: Vec<String> = self.input
            .iter()
            .map(|item| {
                format!("{}:{} (r:{})",
                        item.key,
                        self.format_versions(&item.version),
                        item.rank)
            })
            .collect();

        items.join(", ")
    }

    fn format_versions(&self, versions: &[Version]) -> String {
        if versions.is_empty() {
            return "[]".to_string();
        }

        let version_strs: Vec<String> = versions
            .iter()
            .map(|v| {
                let xmax_str = v.xmax
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "∞".to_string());
                format!("\"{}\"[{}-{}]", v.value, v.xmin, xmax_str)
            })
            .collect();

        format!("[{}]", version_strs.join(", "))
    }

    /// Print tree statistics
    pub fn print_stats(&self) {
        let stats = self.calculate_stats();
        println!("Tree Statistics:");
        println!("├── Total nodes: {}", stats.total_nodes);
        println!("├── Tree height: {}", stats.height);
        println!("├── Total keys: {}", stats.total_keys);
        println!("├── Total versions: {}", stats.total_versions);
        println!("├── Leaf nodes: {}", stats.leaf_nodes);
        println!("└── Internal nodes: {}", stats.internal_nodes);
        println!();
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

        for item in &self.input {
            stats.total_versions += item.version.len();
        }

        if self.children.is_empty() {
            stats.leaf_nodes += 1;
        } else {
            stats.internal_nodes += 1;
            for child_arc in &self.children {
                if let Ok(child) = child_arc.read() {
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
    total_versions: usize,
    leaf_nodes: usize,
    internal_nodes: usize,
}