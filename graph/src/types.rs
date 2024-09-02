use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

pub type NodeId = usize;

pub type EdgeId = usize;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Graph {
    /// All nodes in the graph.
    nodes: HashMap<NodeId, NodeData>,
    /// All edges in the graph.
    edges: HashMap<EdgeId, EdgeData>,
    /// For each node, a list of all edges that have this node as the source.
    nodes_to_outgoing_edges: HashMap<NodeId, Vec<EdgeId>>,
    /// For each node, a list of all edges that have this node as the target.
    /// This is the reverse of `nodes_to_outgoing_edges`.
    nodes_to_incoming_edges: HashMap<NodeId, Vec<EdgeId>>,
    /// The next node ID to be used.
    last_node_id: NodeId,
    /// The next edge ID to be used.
    last_edge_id: EdgeId,
}

impl Graph {
    pub fn add_node(&mut self, node: NodeData) -> NodeId {
        let id = self.fresh_node_id();
        self.nodes.insert(id, node);
        self.nodes_to_outgoing_edges.insert(id, Vec::new());
        self.nodes_to_incoming_edges.insert(id, Vec::new());
        id
    }

    pub fn add_edge(&mut self, edge: EdgeData) -> EdgeId {
        let id = self.fresh_edge_id();
        let EdgeData { from, to, .. } = edge;
        self.edges.insert(id, edge);
        self.nodes_to_outgoing_edges
            .entry(from)
            .or_default()
            .push(id);
        self.nodes_to_incoming_edges.entry(to).or_default().push(id);
        id
    }

    pub fn node<N: Into<NodeId>>(&self, id: N) -> Option<&NodeData> {
        let id = id.into();
        self.nodes.get(&id)
    }

    pub fn node_mut<N: Into<NodeId>>(&mut self, id: N) -> Option<&mut NodeData> {
        let id = id.into();
        self.nodes.get_mut(&id)
    }

    pub fn edge<E: Into<EdgeId>>(&self, id: E) -> Option<&EdgeData> {
        let id = id.into();
        self.edges.get(&id)
    }

    pub fn edge_mut<E: Into<EdgeId>>(&mut self, id: E) -> Option<&mut EdgeData> {
        let id = id.into();
        self.edges.get_mut(&id)
    }

    pub fn node_outgoing_edges(&self, id: NodeId) -> Option<&[EdgeId]> {
        self.nodes_to_outgoing_edges.get(&id).map(|v| v.as_slice())
    }

    pub fn node_incoming_edges(&self, id: NodeId) -> Option<&[EdgeId]> {
        self.nodes_to_incoming_edges.get(&id).map(|v| v.as_slice())
    }

    pub fn node_neighbors(&self, id: NodeId) -> Option<Vec<NodeId>> {
        self.node_outgoing_edges(id).map(|edges| {
            edges
                .iter()
                .map(|&edge_id| self.edges[&edge_id].to)
                .collect()
        })
    }

    pub fn node_children(&self, id: NodeId) -> Option<Vec<NodeId>> {
        self.node_outgoing_edges(id).map(|edges| {
            edges
                .iter()
                .filter(|&&edge_id| self.edges[&edge_id].relation == Relation::IsParentOf)
                .map(|&edge_id| self.edges[&edge_id].to)
                .collect()
        })
    }

    pub fn node_data(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(&id)
    }

    pub fn node_data_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(&id)
    }

    pub fn edge_data(&self, id: EdgeId) -> Option<&EdgeData> {
        self.edges.get(&id)
    }

    pub fn edge_data_mut(&mut self, id: EdgeId) -> Option<&mut EdgeData> {
        self.edges.get_mut(&id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    pub fn edges(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.keys().copied()
    }

    fn fresh_node_id(&mut self) -> NodeId {
        let id = self.last_node_id;
        self.last_node_id += 1;
        id
    }

    fn fresh_edge_id(&mut self) -> EdgeId {
        let id = self.last_edge_id;
        self.last_edge_id += 1;
        id
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeData {
    pub contents: NodeContents,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NodeContents {
    Folder {
        display_name: String,
        path: PathBuf,
    },
    File {
        display_name: String,
        path: PathBuf,
    },
    Item {
        display_name: String,
        moniker: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EdgeData {
    pub from: NodeId,
    pub to: NodeId,
    pub relation: Relation,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// <from> is parent of <to>
    IsParentOf,
}
