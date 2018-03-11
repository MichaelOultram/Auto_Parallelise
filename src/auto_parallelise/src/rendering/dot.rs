use syntax::print::pprust;

use parallel_stages::{dependency_analysis, scheduler};
use self::dependency_analysis::{DependencyTree, DependencyNode};
use self::scheduler::{Schedule, ScheduleTree};

pub fn deptree_to_dot(deptree: &DependencyTree) -> String {
    let mut output = "".to_owned();
    output.push_str("digraph G {\n");
    output.push_str(&subdeptree_to_dot("0", deptree));
    output.push_str("}\n");
    output
}

fn subdeptree_to_dot(prefix: &str, deptree: &DependencyTree) -> String {
    let mut output = "".to_owned();
    let mut prefix_counter = 0;
    for nodeid in 0..deptree.len() {
        let node = &deptree[nodeid];
        // Create a node in dot
        match node {
            &DependencyNode::Expr(ref stmt, _, _) |
            &DependencyNode::Mac(ref stmt, _, _) => {
                output.push_str(&format!("\"{}-{}\" [label=\"{}\"];\n", prefix, nodeid, pprust::stmt_to_string(stmt).replace("\"", "\\\"")));
            },
            &DependencyNode::Block(_, ref subtree, _, _) => {
                // Convert block to dot
                let new_prefix = format!("{}-{}", prefix, prefix_counter);
                prefix_counter += 1;
                output.push_str(&format!("subgraph \"cluster{}\" {{\n", new_prefix));
                output.push_str(&subdeptree_to_dot(&new_prefix, subtree));
                output.push_str("}\n");
            },
            &DependencyNode::ExprBlock(ref stmt, ref subtree, _, _) => {
                output.push_str(&format!("\"{}-{}\" [label=\"{}\"];\n", prefix, nodeid, pprust::stmt_to_string(stmt).replace("\"", "\\\"")));

                // Convert block to dot
                let new_prefix = format!("{}-{}", prefix, prefix_counter);
                prefix_counter += 1;
                //output.push_str(&format!("subgraph \"{}\" {{\n", new_prefix));
                output.push_str(&subdeptree_to_dot(&new_prefix, subtree));
                //output.push_str("}\n");

                output.push_str(&format!("\"{}-{}\" -> \"cluster{}-0\";\n",prefix, nodeid, new_prefix));
            },
        }

        // Add the dependencies
        for dep in node.get_deps() {
            output.push_str(&format!("\"{}-{}\" -> \"{}-{}\";\n", prefix, dep, prefix, nodeid));
        }
    }
    output
}


pub fn schedule_to_dot(schedule: &Schedule) -> String {
    let mut output = "".to_owned();
    output.push_str("digraph G {\n");
    output.push_str(&subschedule_to_dot(schedule));
    output.push_str("}\n");
    output
}

fn subschedule_to_dot(schedule: &Schedule) -> String {
    let mut output = "".to_owned();
    let schlist = schedule.list();
    if schlist.len() > 1 {

    }
    for node in schlist {
        match node {
            &ScheduleTree::Node(ref prereqs, ref tree) => {},
            &ScheduleTree::Block(ref prereqs, ref tree, ref subschedule) => {},
            &ScheduleTree::SyncTo(ref to, ref from, ref env) => {},
        }
    }
    output
}
