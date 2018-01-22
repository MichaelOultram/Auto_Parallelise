use dependency_analysis::{DependencyTree, DependencyNode, StmtID};

pub struct Schedule {

}
#[derive(Debug, PartialEq)]
struct SpanningTree<'a> {
    node: &'a DependencyNode,
    weight: u32,
    children: Vec<SpanningTree<'a>>,
}

impl<'a> SpanningTree<'a> {
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


}

pub fn create_schedule(deptree: &DependencyTree) -> Option<Schedule> {
    // Find all the independent nodes in the current block
    let mut spanning_trees: Vec<SpanningTree> = vec![];
    let mut dependent_nodes = vec![];
    for node in deptree {
        let deps_stmtids = node.get_deps_stmtids(deptree);
        if deps_stmtids.len() == 0 {
            // Independent nodes should create a new spanning_tree
            spanning_trees.push(SpanningTree {
                node: node,
                weight: performance_metric(&node),
                children: vec![],
            });
        } else {
            // Dependent nodes are stored in a list to be added later
            dependent_nodes.push((node, deps_stmtids));
        }
    }

    // Create multiple minimum spanning tree, but each node can only appear once
    minimum_spanning_trees(&mut spanning_trees, &mut dependent_nodes);
    println!("Spanning Trees: {:?}", spanning_trees);
    // TODO: Add in synchronisation lines for missing dependencies

    None
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

        if num_remaining == dependent_nodes.len() {
            panic!("Stuck in an infinite loop");
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
                    tree_node.children.push(SpanningTree {
                        node: node,
                        weight: performance_metric(&node),
                        children: vec![],
                    });
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
