use dependency_analysis::{DependencyTree, DependencyNode, StmtID};

pub struct Schedule<'a> {
    spanning_trees: Vec<SpanningTree<'a>>,
    sync_lines: Vec<(StmtID, StmtID)>, // Prerequisite dependency, Node to wait for the dependency
}
#[derive(Debug, PartialEq)]
struct SpanningTree<'a> {
    node: &'a DependencyNode,
    weight: u32,
    children: Vec<SpanningTree<'a>>,
}

impl<'a> SpanningTree<'a> {

}


impl<'a> SpanningTree<'a> {
    pub fn new(node: &'a DependencyNode) -> Self {
        SpanningTree {
            node: node,
            weight: performance_metric(&node),
            children: vec![],
        }
    }

    fn get_by_stmtid(&mut self, stmtid: StmtID) -> Option<&mut SpanningTree<'a>> {
        if self.node.get_stmtid() == stmtid {
            return Some(self);
        } else {
            for child in &mut self.children {
                let result = child.get_by_stmtid(stmtid);
                if let Some(_) = result {
                    return result;
                }
            }
            None
        }
    }

    fn add_child(&mut self, node:&'a DependencyNode) {
        self.children.push(SpanningTree {
            node: node,
            weight: self.weight + performance_metric(&node),
            children: vec![],
        });
    }

    pub fn to_string(&self, indent: u32) -> String {
        let mut output = "".to_owned();
        for _ in 0..indent {
            output.push_str("    ");
        }
        output.push_str(&format!("[{}] {:?}", self.weight, self.node));
        for c in &self.children {
            output.push_str("\n");
            output.push_str(&c.to_string(indent + 1));
        }
        output
    }
}

pub fn create_schedule(deptree: &DependencyTree) -> Schedule {
    // Find all the independent nodes in the current block
    let mut spanning_trees: Vec<SpanningTree> = vec![];
    let mut dependent_nodes = vec![];
    for node in deptree {
        let deps_stmtids = node.get_deps_stmtids(deptree);
        if deps_stmtids.len() == 0 {
            // Independent nodes should create a new spanning_tree
            spanning_trees.push(SpanningTree::new(node));
        } else {
            // Dependent nodes are stored in a list to be added later
            dependent_nodes.push((node, deps_stmtids));
        }
    }

    // Create multiple minimum spanning tree, but each node can only appear once
    minimum_spanning_trees(&mut spanning_trees, &mut dependent_nodes);
    println!("SPANNING TREES:");
    for tree in &spanning_trees {
        println!("{}\n", tree.to_string(0));
    }
    // TODO: Add in synchronisation lines for missing dependencies

    Schedule {
        spanning_trees: spanning_trees,
        sync_lines: vec![],
    }
}

fn minimum_spanning_trees<'a>(spanning_trees: &mut Vec<SpanningTree<'a>>,
                              dependent_nodes: &mut Vec<(&'a DependencyNode, Vec<StmtID>)>) {
    // TODO
    //unimplemented!()
    let mut num_remaining;
    while dependent_nodes.len() > 0 {
        num_remaining = dependent_nodes.len();
        add_single_deps(spanning_trees, dependent_nodes);
        // TODO: Check for nodes with all their dependencies on the spanning_tree
        // TODO: Add the node to the longest dependency

        // Check to see if nothing was added in the last iteration
        if num_remaining == dependent_nodes.len() {
            //TODO: Should panic instead
            println!("Stuck in an infinite loop");
            return
        }
    }
}

fn add_single_deps<'a>(spanning_trees: &mut Vec<SpanningTree<'a>>,
                       dependent_nodes: &mut Vec<(&'a DependencyNode, Vec<StmtID>)>) {
    // Add all nodes with a single dependency to the tree if their dependent node is on the tree
    // Look at all the nodes and their dependencies
    dependent_nodes.retain(|&(ref node, ref deps_stmtids)| {
        // If they have a single dependency
        let mut keep_node = true;
        if deps_stmtids.len() == 1 {
            let dep_stmtid = deps_stmtids[0];
            // Find the tree nodes that the dependency matches
            for tree in spanning_trees.iter_mut() {
                let result = tree.get_by_stmtid(dep_stmtid);
                if let Some(tree_node) = result {
                    // If it is found, add the node as a child
                    tree_node.add_child(node);
                    keep_node = false;
                }
            }
        }
        keep_node
    })
}

fn performance_metric(node: &DependencyNode) -> u32 {
    match node {
        &DependencyNode::Expr(_, _, _) => 1,
        &DependencyNode::Block(_, ref nodes,_) => {
            let mut total = 0;
            for node in nodes {
                total += performance_metric(node);
            }
            total
        },
        &DependencyNode::Mac(_, _, _) => 1,
    }
}
