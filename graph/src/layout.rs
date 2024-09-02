use std::collections::HashMap;

use crate::{EdgeId, Graph, NodeId};

const IDEAL_SPRING_LENGTH: f64 = 50.0;

#[derive(Debug)]
pub struct Layout {
    pub rects: HashMap<NodeId, kurbo::Rect>,
    pub lines: HashMap<EdgeId, kurbo::Line>,
}

impl Layout {
    pub fn compute(graph: &Graph) -> Self {
        let mut layout = initial_layout(graph);

        apply_forces(graph, &mut layout, 0.1, 50000);

        layout_edges(graph, &mut layout);

        layout
    }
}

fn apply_forces(graph: &Graph, layout: &mut Layout, threshold: f64, max_iterations: usize) {
    let initial_temperature: f64 = 1.0;
    let mut step = 1;
    let mut forces = HashMap::new();

    while step < max_iterations {
        let mut max_force = kurbo::Vec2::new(0.0, 0.0);

        for node_id in graph.nodes() {
            let force = compute_force(graph, layout, node_id);
            let delta = cooling_factor(initial_temperature, step, max_iterations);
            forces.insert(node_id, delta * force);
            if force.length() > max_force.length() {
                max_force = force;
            }
        }

        for node_id in graph.nodes() {
            let rect = layout.rects[&node_id];
            let new_rect =
                kurbo::Rect::from_origin_size(rect.origin() + forces[&node_id], rect.size());
            layout.rects.insert(node_id, new_rect);
        }

        if max_force.length() < threshold {
            break;
        }

        if step % 1000 == 0 {
            println!("Step: {}, max force: {}", step, max_force.length());
        }

        step += 1;
    }
}

fn cooling_factor(initial_temperature: f64, step: usize, max_iterations: usize) -> f64 {
    let alpha = 1.0;
    let beta = 1.0;
    let gamma = 1.0;

    initial_temperature * alpha
        / (1.0 + beta * initial_temperature * step as f64 / max_iterations as f64).powf(gamma)
}

fn compute_force(graph: &Graph, layout: &Layout, node_id: NodeId) -> kurbo::Vec2 {
    let repulsive = graph
        .nodes()
        .filter(|&other_id| other_id != node_id)
        .map(|other_id| repulsive_force(layout, node_id, other_id))
        .reduce(|u, v| u + v)
        .unwrap_or_default();

    let attractive = graph
        .node_outgoing_edges(node_id)
        .unwrap()
        .iter()
        .map(|&edge_id| attractive_force(layout, node_id, graph.edge(edge_id).unwrap().to))
        .reduce(|u, v| u + v)
        .unwrap_or_default();

    repulsive + attractive
}

/// Compute the repulsive force between two nodes.
fn repulsive_force(layout: &Layout, u: NodeId, v: NodeId) -> kurbo::Vec2 {
    let pos_u = layout.rects[&u].center();
    let pos_v = layout.rects[&v].center();

    // Prevent division by zero
    let distance = pos_u.distance(pos_v).max(1e-6);
    let force = IDEAL_SPRING_LENGTH.powi(2) / distance * (pos_u - pos_v) / distance;

    if !force.is_finite() {
        return kurbo::Vec2::ZERO;
    }

    force
}

fn attractive_force(layout: &Layout, u: NodeId, v: NodeId) -> kurbo::Vec2 {
    let pos_u = layout.rects[&u].center();
    let pos_v = layout.rects[&v].center();

    let distance = pos_u.distance(pos_v);
    let force = (distance.powi(2) / IDEAL_SPRING_LENGTH) * (pos_v - pos_u);

    // Limit the force to a maximum magnitude to prevent overflow
    let max_force_magnitude = 1000.0;
    let clamped_force = kurbo::Vec2::new(
        force.x.min(max_force_magnitude).max(-max_force_magnitude),
        force.y.min(max_force_magnitude).max(-max_force_magnitude),
    );

    if !clamped_force.is_finite() {
        return kurbo::Vec2::ZERO;
    }

    clamped_force
}

fn initial_layout(graph: &Graph) -> Layout {
    let mut layout = Layout {
        rects: HashMap::new(),
        lines: HashMap::new(),
    };

    let mut x = 0.0;
    let mut y = 0.0;

    for node_id in graph.nodes() {
        layout.rects.insert(
            node_id,
            kurbo::Rect::from_origin_size((x, y), (64.0, 100.0)),
        );

        x += 150.0;
        y += 150.0;
    }

    layout_edges(graph, &mut layout);

    layout
}

fn layout_edges(graph: &Graph, layout: &mut Layout) {
    for edge_id in graph.edges() {
        layout.lines.insert(
            edge_id,
            kurbo::Line::new(
                layout.rects[&graph.edge(edge_id).unwrap().from].center(),
                layout.rects[&graph.edge(edge_id).unwrap().to].center(),
            ),
        );
    }
}
