use crate::models::ImpactNode;

/// Print an impact tree as ASCII art.
pub fn print_tree(node: &ImpactNode, depth: usize) {
    if depth == 0 {
        println!("{}:{}", node.node_type, node.node_id);
    }

    let total = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i == total - 1;
        let prefix = build_prefix(depth, is_last);
        let connector = if is_last { "└── " } else { "├── " };

        println!(
            "{}{}{}:{}",
            prefix, connector, child.node_type, child.node_id
        );

        if !child.children.is_empty() {
            let child_prefix = if is_last { "    " } else { "│   " };
            print_subtree(child, depth, child_prefix);
        }
    }
}

fn print_subtree(node: &ImpactNode, parent_depth: usize, parent_prefix: &str) {
    let total = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i == total - 1;
        let prefix = build_prefix_with_parent(parent_depth, parent_prefix);
        let connector = if is_last { "└── " } else { "├── " };

        println!(
            "{}{}{}:{}",
            prefix, connector, child.node_type, child.node_id
        );

        if !child.children.is_empty() {
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            print_subtree_deep(child, &child_prefix);
        }
    }
}

fn print_subtree_deep(node: &ImpactNode, prefix: &str) {
    let total = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };

        println!(
            "{}{}{}:{}",
            prefix, connector, child.node_type, child.node_id
        );

        if !child.children.is_empty() {
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            print_subtree_deep(child, &child_prefix);
        }
    }
}

fn build_prefix(depth: usize, _is_last: bool) -> String {
    "    ".repeat(depth)
}

fn build_prefix_with_parent(depth: usize, parent_prefix: &str) -> String {
    let base = "    ".repeat(depth);
    format!("{}{}", base, parent_prefix)
}

/// Print a simple path as ASCII arrows.
#[allow(dead_code)]
pub fn print_path(path: &[String]) {
    if path.is_empty() {
        return;
    }

    for (i, node) in path.iter().enumerate() {
        if i == 0 {
            println!("  {}", node);
        } else {
            println!("  → {}", node);
        }
    }
}
