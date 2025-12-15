use bevy::{
    ecs::{entity::Entity, resource::Resource},
    math::Vec2,
    platform::collections::{HashMap, HashSet},
};
use petgraph::{
    Graph, Undirected,
    graph::{EdgeIndex, NodeIndex},
};
use rand::Rng;

#[derive(Resource)]
pub struct ComputerGraph(pub Graph<ComputerNode, (), Undirected>);

#[derive(Clone, Copy)]
pub struct ComputerNode {
    pub position: Vec2,
}

#[derive(Resource, Default)]
pub struct GraphEntityMap {
    pub nodes: HashMap<NodeIndex, Entity>,
    pub edges: HashMap<EdgeIndex, Entity>,
}

#[derive(Resource, Default)]
pub struct InteractionState {
    pub selected_source: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,
    pub path: Vec<NodeIndex>,
}

#[derive(Resource, Default)]
pub struct FlowMap {
    pub flows: HashMap<NodeIndex, HashSet<NodeIndex>>,
}

impl ComputerGraph {
    pub fn random() -> Self {
        const NODE_COUNT: usize = 30;
        const ATTEMPTS: usize = 20;
        const MIN_DIST: f32 = 0.2;
        const CONNECT_DIST: f32 = 0.45;

        let mut graph = Graph::new_undirected();
        let mut rng = rand::rng();

        let mut positions: Vec<Vec2> = Vec::with_capacity(NODE_COUNT);
        'outer: for _ in 0..(NODE_COUNT * ATTEMPTS) {
            if positions.len() >= NODE_COUNT {
                break;
            }
            let candidate = Vec2::new(rng.random_range(-0.8..0.8), rng.random_range(-0.8..0.8));

            for pos in &positions {
                if pos.distance(candidate) < MIN_DIST {
                    continue 'outer;
                }
            }
            positions.push(candidate);
        }

        let node_indices: Vec<NodeIndex> = positions
            .iter()
            .map(|&pos| graph.add_node(ComputerNode { position: pos }))
            .collect();

        for i in 0..node_indices.len() {
            for j in (i + 1)..node_indices.len() {
                let idx_a = node_indices[i];
                let idx_b = node_indices[j];
                let pos_a = graph[idx_a].position;
                let pos_b = graph[idx_b].position;
                if pos_a.distance(pos_b) < CONNECT_DIST {
                    graph.add_edge(idx_a, idx_b, ());
                }
            }
        }

        loop {
            let mut components: Vec<Vec<NodeIndex>> = Vec::new();
            let mut visited = HashSet::new();

            for &node in &node_indices {
                if !visited.contains(&node) {
                    let mut component = Vec::new();
                    let mut bfs = petgraph::visit::Bfs::new(&graph, node);
                    while let Some(visited_node) = bfs.next(&graph) {
                        visited.insert(visited_node);
                        component.push(visited_node);
                    }
                    components.push(component);
                }
            }

            if components.len() <= 1 {
                break;
            }

            let mut min_dist = f32::MAX;
            let mut best_edge = None;
            let island_a = &components[0];

            for island_b in components.iter().skip(1) {
                for &node_a in island_a {
                    for &node_b in island_b {
                        let dist = graph[node_a].position.distance(graph[node_b].position);
                        if dist < min_dist {
                            min_dist = dist;
                            best_edge = Some((node_a, node_b));
                        }
                    }
                }
            }
            if let Some((u, v)) = best_edge {
                graph.add_edge(u, v, ());
            } else {
                break;
            }
        }
        Self(graph)
    }
}
