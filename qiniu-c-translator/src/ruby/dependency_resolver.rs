//! 使用拓扑算法的方式对依赖进行排序，使被依赖的类型声明在前面，依赖的类型声明在后面。
//! 这样 Ruby 代码在执行的时候，就不会碰到出现依赖的，但是在后面才被定义的类型

use crate::utils::CodeGenerator;
use std::{collections::HashMap, fmt};
use tree_mem_sort::sort_dag;

#[derive(Default)]
pub(super) struct DependenciesResolver {
    nodes: Vec<Node>,
    names_map: HashMap<String, usize>,
}

struct Node {
    index: usize,
    name: String,
    ast_node: Option<Box<dyn CodeGenerator>>,
    depend_on_indices: Vec<usize>,
    be_depended_on_indices: Vec<usize>,
}

impl DependenciesResolver {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn insert(&mut self, name: String, ast_node: Box<dyn CodeGenerator>, depend_on: Vec<String>) {
        let new_node_index = self.get_or_insert_node(name, Some(ast_node));
        for depended_name in depend_on {
            let (depended_index, depended_node) = if let Some(&depended_index) = self.names_map.get(&depended_name) {
                let depended_node = self.nodes.get_mut(depended_index).unwrap();
                (depended_index, depended_node)
            } else {
                let new_depended_node_index = self.get_or_insert_node(depended_name, None);
                let new_depended_node = self.nodes.get_mut(new_depended_node_index).unwrap();
                (new_depended_node_index, new_depended_node)
            };
            depended_node.be_depended_on_indices.push(new_node_index);
            self.nodes
                .get_mut(new_node_index)
                .unwrap()
                .depend_on_indices
                .push(depended_index);
        }
    }

    fn get_or_insert_node(&mut self, name: String, ast_node: Option<Box<dyn CodeGenerator>>) -> usize {
        if let Some(&index) = self.names_map.get(&name) {
            let node = self.nodes.get_mut(index).unwrap();
            if ast_node.is_some() {
                node.ast_node = ast_node;
            }
            return node.index;
        }
        let index = self.nodes.len();
        let node = Node {
            index,
            name: name.to_owned(),
            ast_node,
            depend_on_indices: Default::default(),
            be_depended_on_indices: Default::default(),
        };
        self.nodes.push(node);
        self.names_map.insert(name, index);
        index
    }

    pub(super) fn resolve(mut self) -> Vec<Box<dyn CodeGenerator>> {
        sort_dag(
            &mut self.nodes,
            |node| &mut node.depend_on_indices,
            |node| &mut node.be_depended_on_indices,
        );
        self.nodes.into_iter().map(|node| node.ast_node.unwrap()).collect()
    }
}

impl fmt::Debug for DependenciesResolver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug_set = f.debug_set();
        for node in self.nodes.iter() {
            debug_set.entry(&NodeForDebug {
                index: node.index,
                name: &node.name,
                has_ast_node: node.ast_node.is_some(),
                depend_on_indices: &node.depend_on_indices,
                be_depended_on_indices: &node.be_depended_on_indices,
            });
        }
        debug_set.finish()
    }
}

#[derive(Debug)]
struct NodeForDebug<'a> {
    index: usize,
    name: &'a str,
    has_ast_node: bool,
    depend_on_indices: &'a [usize],
    be_depended_on_indices: &'a [usize],
}
