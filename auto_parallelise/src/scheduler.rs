use dependency_analysis::{DependencyTree, DependencyNode};

pub struct Schedule {

}

struct SpanningTree {
    node: DependencyNode,
    weight: u32,
    children: Vec<SpanningTree>,
}

pub fn create_schedule(deptree: &DependencyTree) -> Schedule {
    // Find all the independent nodes in the current block
    let mut independent_nodes = vec![];
    let mut dependent_nodes = vec![];
    for node in deptree {
        match node {
            &DependencyNode::Expr(_, ref deps)  |
            &DependencyNode::Block(_, ref deps) |
            &DependencyNode::Mac(ref deps) => {
                if deps.len() == 0 {
                    independent_nodes.push(node);
                } else {
                    dependent_nodes.push(node);
                }
            },
        }
    }

    // Create multiple minimum spanning tree, but each node can only appear once
    let spanning_trees = minimum_spanning_trees(independent_nodes, dependent_nodes);

    // TODO: Add in synchronisation lines for missing dependencies

    unimplemented!()
}

fn minimum_spanning_trees(independent_nodes: Vec<&DependencyNode>,
                         dependent_nodes: Vec<&DependencyNode>) -> Vec<SpanningTree> {
    unimplemented!()
}


fn performance_metric(node: &DependencyNode) -> u32 {
    match node {
        &DependencyNode::Expr(_,_) => 1,
        &DependencyNode::Block(ref nodes,_) => {
            let mut total = 0;
            for node in nodes {
                total += performance_metric(node);
            }
            total
        },
        &DependencyNode::Mac(_) => 1,
    }
}
