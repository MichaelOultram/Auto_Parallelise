use dependency_analysis::{DependencyTree, DependencyNode, StmtID};

#[derive(Debug, PartialEq, Serialize)]
pub struct Schedule<'a> {
    schedule_trees: Vec<ScheduleTree<'a>>,
    sync_lines: Vec<(StmtID, StmtID)>, // Prerequisite dependency, Node to wait for the dependency
}

#[derive(Debug, PartialEq, Serialize)]
enum ScheduleTree<'a> {
    Node(SpanningTree<'a>),
    Block(SpanningTree<'a>, Schedule<'a>),
}

impl <'a>ScheduleTree<'a>{
    pub fn new(node: &'a DependencyNode) -> Self {
        match node {
            &DependencyNode::Expr(_, _) |
            &DependencyNode::Mac(_, _) => {
                ScheduleTree::Node(SpanningTree::new(node, 0)) //TODO: get extra weight
            },

            &DependencyNode::Block(_, ref tree, _) |
            &DependencyNode::ExprBlock(_, ref tree, _) => {
                ScheduleTree::Block(SpanningTree::new(node, 0), create_schedule(tree))
            },
        }
    }

    pub fn get_spanning_tree(&self) -> &SpanningTree<'a> {
        match self {
            &ScheduleTree::Node(ref tree) |
            &ScheduleTree::Block(ref tree, _) => tree,
        }
    }

    pub fn get_spanning_tree_mut(&mut self) -> &mut SpanningTree<'a> {
        match self {
            &mut ScheduleTree::Node(ref mut tree) |
            &mut ScheduleTree::Block(ref mut tree, _) => tree,
        }
    }

    pub fn to_string(&self, indent: u32) -> String {
        let mut output = "".to_owned();
        for _ in 0..indent {
            output.push_str("    ");
        }
        match self {
            &ScheduleTree::Node(ref tree) => {}
            &ScheduleTree::Block(ref tree, ref schedule) => {},
        }

        output.push_str(&format!("{:?}", self.get_spanning_tree().to_string(0)));
        output
    }
}

#[derive(Debug, PartialEq, Serialize)]
struct SpanningTree<'a> {
    node: &'a DependencyNode,
    weight: u32,
    children: Vec<ScheduleTree<'a>>,
}

impl<'a> SpanningTree<'a> {
    pub fn new(node: &'a DependencyNode, extra_weight: u32) -> Self {
        SpanningTree {
            node: node,
            weight: extra_weight + performance_metric(&node),
            children: vec![],
        }
    }

    fn get_by_stmtid(&mut self, stmtid: StmtID) -> Option<&mut SpanningTree<'a>> {
        if self.node.get_stmtid() == stmtid {
            return Some(self);
        } else {
            for child in &mut self.children {
                let result = child.get_spanning_tree_mut().get_by_stmtid(stmtid);
                if let Some(_) = result {
                    return result;
                }
            }
            None
        }
    }

    fn add_child(&mut self, node:&'a DependencyNode) {
        self.children.push(ScheduleTree::new(node));
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
    println!("create_schedule()");
    // Find all the independent nodes in the current block
    let mut schedule_trees: Vec<ScheduleTree> = vec![];
    let mut dependent_nodes = vec![];
    for node in deptree {
        let deps_stmtids = node.get_deps_stmtids(deptree);
        if deps_stmtids.len() == 0 {
            // Independent nodes should create a new spanning_tree
            schedule_trees.push(ScheduleTree::new(node));
        } else {
            // Dependent nodes are stored in a list to be added later
            dependent_nodes.push((node, deps_stmtids));
        }
    }

    // Create multiple maximum spanning tree, but each node can only appear once
    maximum_spanning_trees(&mut schedule_trees, &mut dependent_nodes);

    // TODO: Add in synchronisation lines for missing dependencies
    Schedule {
        schedule_trees: schedule_trees,
        sync_lines: vec![],
    }
}

fn maximum_spanning_trees<'a>(schedule_trees: &mut Vec<ScheduleTree<'a>>,
                              dependent_nodes: &mut Vec<(&'a DependencyNode, Vec<StmtID>)>) {
    // TODO
    //unimplemented!()
    let mut num_remaining;
    while dependent_nodes.len() > 0 {
        num_remaining = dependent_nodes.len();
        add_single_deps(schedule_trees, dependent_nodes);
        // TODO: Check for nodes with all their dependencies on the spanning_tree
        // TODO: Add the node to the longest dependency

        dependent_nodes.retain(|&(ref node, ref deps_stmtids)| {
            // If they have a single dependency
            let mut best_nodes_ids = vec![]; // (TreeID,Weight)
            let mut all_deps_added = true;
            let mut keep_node = true;
            for dep_stmtid in deps_stmtids {
                // Find the tree nodes that the dependency matches
                let mut tree_id_pair: Option<(StmtID,usize,u32)> = None;
                for tree_id in 0..schedule_trees.len() {
                    let result = schedule_trees[tree_id].get_spanning_tree_mut().get_by_stmtid(*dep_stmtid);
                    if let Some(_) = result {
                        tree_id_pair = Some((*dep_stmtid,tree_id,0));//TODO: add weight
                    }
                }
                if let Some(pair) = tree_id_pair {
                    best_nodes_ids.push(pair);
                } else {
                    all_deps_added = false;
                }
            }

            // Check that all dependencies
            if all_deps_added {
                // Find largest weight
                let mut best_node_id: Option<(StmtID,usize,u32)> = None;
                for node_id in best_nodes_ids {
                    if let Some((_,_,best_weight)) = best_node_id {
                        let (_,_,weight) = node_id ;
                        if best_weight < weight {
                            best_node_id = Some(node_id);
                        }
                    } else {
                        best_node_id = Some(node_id);
                    }
                }
                // Add node to best branch
                if let Some((stmtid, tree_id, _)) = best_node_id {
                    let result = schedule_trees[tree_id].get_spanning_tree_mut().get_by_stmtid(stmtid);
                    if let Some(tree_node) = result {
                        tree_node.add_child(node);
                        keep_node = false;
                    }
                }
            }
            keep_node
        });

        // Check to see if nothing was added in the last iteration
        if num_remaining == dependent_nodes.len() {
            panic!("Stuck in an infinite loop");
        }
    }
}

fn add_single_deps<'a>(schedule_trees: &mut Vec<ScheduleTree<'a>>,
                       dependent_nodes: &mut Vec<(&'a DependencyNode, Vec<StmtID>)>) {
    // Add all nodes with a single dependency to the tree if their dependent node is on the tree
    // Look at all the nodes and their dependencies
    dependent_nodes.retain(|&(ref node, ref deps_stmtids)| {
        // If they have a single dependency
        let mut keep_node = true;
        if deps_stmtids.len() == 1 {
            let dep_stmtid = deps_stmtids[0];
            // Find the tree nodes that the dependency matches
            for tree in schedule_trees.iter_mut() {
                let result = tree.get_spanning_tree_mut().get_by_stmtid(dep_stmtid);
                if let Some(tree_node) = result {
                    // If it is found, add the node as a child
                    tree_node.add_child(node);
                    keep_node = false;
                }
            }
        }
        keep_node
    });
}

fn performance_metric(node: &DependencyNode) -> u32 {
    match node {
        &DependencyNode::Expr(_, _) => 1,
        &DependencyNode::ExprBlock(_, ref nodes,_) |
        &DependencyNode::Block(_, ref nodes,_) => {
            let mut total = 0;
            for node in nodes {
                total += performance_metric(node);
            }
            total
        },
        &DependencyNode::Mac(_, _) => 1,
    }
}
