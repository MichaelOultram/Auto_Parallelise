use dependency_analysis::{DependencyTree, DependencyNode, StmtID, Environment, InOutEnvironment};
use serde::ser::{Serialize, Serializer, SerializeStruct};

#[derive(Debug, PartialEq, Serialize)]
pub struct Schedule<'a>(Vec<ScheduleTree<'a>>);
impl<'a> Schedule<'a> {
    pub fn get_all_synclines(&self) -> Vec<(StmtID, StmtID, &Environment)> {
        let mut synclines = vec![];
        for tree in &(self.0) {
            synclines.append(&mut tree.get_all_synclines());
        }
        synclines
    }

    pub fn get_env(&self) -> InOutEnvironment {
        let (mut inenv, mut outenv) = (Environment::empty(), Environment::empty());
        for schtree in &(self.0) {
            if let Some(span_tree) = schtree.get_spanning_tree() {
                let &(ref subinenv, ref suboutenv) = span_tree.node.get_env();
                outenv.remove_env(subinenv.clone());
                inenv.merge(subinenv.clone()); // TODO: May add a dependency which does not exist
                outenv.merge(suboutenv.clone());
            }
        }
        (inenv, outenv)
    }

    pub fn list(&self) -> &Vec<ScheduleTree<'a>> {
        &self.0
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ScheduleTree<'a> {
    // Prerequisite dependencies, Current Statement + Children
    Node(Vec<StmtID>, SpanningTree<'a>),
    // Prerequisite dependencies, Current Statement + Children, Inner Block Schedule
    Block(Vec<StmtID>, SpanningTree<'a>, Schedule<'a>),
    // Node to wait for the dependency
    SyncTo(StmtID, StmtID, Environment),
}

impl<'a> ScheduleTree<'a>{
    fn new(prereqs: Vec<StmtID>, node: &'a DependencyNode) -> Self {
        if prereqs.len() > 0 {
            eprintln!("Got a prereq: {:?}, for node {:?}", prereqs, node);
        }
        match node {
            &DependencyNode::Expr(_, _, _) |
            &DependencyNode::Mac(_, _, _) => {
                ScheduleTree::Node(prereqs, SpanningTree::new(node, 0)) //TODO: get extra weight
            },

            &DependencyNode::Block(_, ref tree, _, _) |
            &DependencyNode::ExprBlock(_, ref tree, _, _) => {
                ScheduleTree::Block(prereqs, SpanningTree::new(node, 0), create_schedule(tree))
            },
        }
    }


    pub fn get_spanning_tree(&self) -> Option<&SpanningTree<'a>> {
        match self {
            &ScheduleTree::Node(_, ref tree) |
            &ScheduleTree::Block(_, ref tree, _) => Some(tree),
            _ => None,
        }
    }

    pub fn get_spanning_tree_mut(&mut self) -> Option<&mut SpanningTree<'a>> {
        match self {
            &mut ScheduleTree::Node(_, ref mut tree) |
            &mut ScheduleTree::Block(_, ref mut tree, _) => Some(tree),
            _ => None,
        }
    }

    fn get_all_synclines(&self) -> Vec<(StmtID, StmtID, &Environment)> {
        let mut synclines = vec![];
        match self {
            &ScheduleTree::Node(_, ref tree) |
            &ScheduleTree::Block(_, ref tree, _) => {
                for child in &tree.children {
                    synclines.append(&mut child.get_all_synclines())
                }
            },
            &ScheduleTree::SyncTo(from, to, ref env) => synclines.push((from, to, env)),
        }
        synclines
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SpanningTree<'a> {
    pub node: &'a DependencyNode,
    pub weight: u32,
    pub children: Vec<ScheduleTree<'a>>,
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
                if let Some(child_tree) = child.get_spanning_tree_mut() {
                    let result = child_tree.get_by_stmtid(stmtid);
                    if let Some(_) = result {
                        return result;
                    }
                }
            }
            None
        }
    }

    fn add_child(&mut self, prereqs: Vec<StmtID>, node:&'a DependencyNode) {
        self.children.push(ScheduleTree::new(prereqs, node));
    }

    fn add_sync_to(&mut self, pre: StmtID, node: StmtID, env: Environment) {
        self.children.push(ScheduleTree::SyncTo(pre, node, env));
    }

}

pub fn create_schedule(deptree: &DependencyTree) -> Schedule {
    eprintln!("create_schedule()");
    // Find all the independent nodes in the current block
    let mut schedule_trees: Vec<ScheduleTree> = vec![];
    let mut dependent_nodes = vec![];
    for node in deptree {
        let deps_stmtids = node.get_deps_stmtids(deptree);
        if deps_stmtids.len() == 0 {
            // Independent nodes should create a new spanning_tree
            schedule_trees.push(ScheduleTree::new(vec![], node));
        } else {
            // Dependent nodes are stored in a list to be added later
            dependent_nodes.push((node, deps_stmtids));
        }
    }

    // Create multiple maximum spanning tree, but each node can only appear once
    maximum_spanning_trees(&mut schedule_trees, &mut dependent_nodes);

    Schedule(schedule_trees)
}

fn maximum_spanning_trees<'a>(schedule_trees: &mut Vec<ScheduleTree<'a>>,
                              dependent_nodes: &mut Vec<(&'a DependencyNode, Vec<StmtID>)>) {
    let mut num_remaining;
    while dependent_nodes.len() > 0 {
        num_remaining = dependent_nodes.len();
        eprintln!("num_remaining: {}", num_remaining);

        // Check for nodes with all their dependencies on the spanning_tree
        // Add the node to the longest dependency

        dependent_nodes.retain(|&(ref node, ref deps_stmtids)| {
            let node_envin = node.get_env().0.clone();
            eprintln!("node_envin: {:?}", node_envin);

            // If they have a single dependency
            let mut best_nodes_ids = vec![]; // (TreeID,Weight)
            let mut all_deps_added = true;
            let mut keep_node = true;
            for dep_stmtid in deps_stmtids {
                // Find the tree nodes that the dependency matches
                let mut tree_id_pair: Option<(StmtID,usize,u32)> = None;
                for tree_id in 0..schedule_trees.len() {
                    if let Some(child_tree) = schedule_trees[tree_id].get_spanning_tree_mut() {
                        let result = child_tree.get_by_stmtid(*dep_stmtid);
                        if let Some(_) = result {
                            tree_id_pair = Some((*dep_stmtid,tree_id,0));//TODO: add weight
                        }
                    } else {
                        panic!();
                    }
                }
                if let Some(pair) = tree_id_pair {
                    best_nodes_ids.push(pair);
                } else {
                    all_deps_added = false;
                }
            }

            // Check that all dependencies are in the tree
            if all_deps_added {
                // Find largest weight
                let mut best_node_id: Option<(StmtID,usize,u32)> = None;
                for node_id in &best_nodes_ids {
                    if let Some((_,_,best_weight)) = best_node_id {
                        let &(_,_,weight) = node_id ;
                        if best_weight < weight {
                            best_node_id = Some(*node_id);
                        }
                    } else {
                        best_node_id = Some(*node_id);
                    }
                }

                if let Some((best_stmtid, best_tree_id, _)) = best_node_id {
                    let mut prereqs = vec![];
                    // Add sync lines for the other dependencies
                    for &(node_stmtid, node_tree_id, _) in &best_nodes_ids {
                        // Check that this node is not the best node
                        if node_stmtid != best_stmtid {
                            // Get the dependency node on the tree
                            if let Some(child_tree) = schedule_trees[node_tree_id].get_spanning_tree_mut() {
                                let result = child_tree.get_by_stmtid(node_stmtid);
                                if let Some(tree_node) = result {
                                    // Get outenv for tree_node
                                    let &(_, ref treeoutenv) = tree_node.node.get_env();
                                    eprintln!("tree_node: {:?}", tree_node.node);
                                    eprintln!("treeoutenv: {:?}", treeoutenv);
                                    // Remove all the elements that this satisfes
                                    let mut diff_env = treeoutenv.clone();
                                    diff_env.remove_env(node_envin.clone());
                                    eprintln!("treeoutenv-node_envin: {:?}", diff_env);
                                    // Want to keep removed elements from diff_env
                                    let mut sync_env = treeoutenv.clone();
                                    sync_env.remove_env(diff_env);
                                    eprintln!("sync_env: {:?}", sync_env);
                                    tree_node.add_sync_to(node_stmtid, node.get_stmtid(), sync_env);
                                    prereqs.push(node_stmtid);
                                } else {
                                    panic!();
                                }
                            } else {
                                panic!();
                            }
                        }
                    }
                    assert!(prereqs.len() + 1 == best_nodes_ids.len());
                    if prereqs.len() > 0 {
                        eprintln!("Had prereqs: {:?}", prereqs);
                    }

                    // Add node to best branch
                    if let Some(child_tree) = schedule_trees[best_tree_id].get_spanning_tree_mut() {
                        let result = child_tree.get_by_stmtid(best_stmtid);
                        if let Some(tree_node) = result {
                            tree_node.add_child(prereqs, node);
                            assert!(keep_node);
                            keep_node = false;
                        } else {
                            panic!();
                        }
                    }

                    assert!(!keep_node);
                } else {
                    panic!();
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

fn performance_metric(node: &DependencyNode) -> u32 {
    match node {
        &DependencyNode::Expr(_, _, _) => 1,
        &DependencyNode::ExprBlock(_, ref nodes,_, _) |
        &DependencyNode::Block(_, ref nodes,_, _) => {
            let mut total = 1;
            for node in nodes {
                total += performance_metric(node);
            }
            total
        },
        &DependencyNode::Mac(_, _, _) => 1,
    }
}
