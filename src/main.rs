use bevy::{camera::ScalingMode, platform::collections::HashSet, prelude::*};
use petgraph::{Graph, Undirected, graph::NodeIndex, visit::Bfs};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_computer_graph, draw_computer_graph).chain())
        .run();
}

#[derive(Resource)]
struct ComputerGraph(Graph<ComputerNode, (), Undirected>);

impl ComputerGraph {
    fn random() -> Self {
        const NODE_COUNT: usize = 30;
        const ATTEMPTS: usize = 20;
        const MIN_DIST: f32 = 0.2;
        const CONNECT_DIST: f32 = 0.4;

        let mut graph = Graph::new_undirected();
        let mut rng = rand::rng();

        let mut positions: Vec<Vec2> = Vec::with_capacity(NODE_COUNT);
        'outer: for _ in 0..(NODE_COUNT * ATTEMPTS) {
            if positions.len() >= NODE_COUNT {
                break;
            }

            let candidate = Vec2::new(rng.random_range(-0.9..0.9), rng.random_range(-0.9..0.9));

            for pos in &positions {
                if pos.distance(candidate) < MIN_DIST {
                    continue 'outer;
                }
            }

            positions.push(candidate);
        }

        let node_indices: Vec<NodeIndex> = positions
            .iter()
            .map(|&pos| {
                graph.add_node(ComputerNode {
                    position: pos,
                    color: Color::WHITE,
                })
            })
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
                    let mut bfs = Bfs::new(&graph, node);
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

#[derive(Clone, Copy)]
struct ComputerNode {
    position: Vec2,
    color: Color,
}

impl ComputerNode {
    fn new(position: Vec2, color: Color) -> Self {
        ComputerNode { position, color }
    }
}

fn setup_computer_graph(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 2.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    commands.insert_resource(ComputerGraph::random());
}

fn draw_computer_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    graph: Res<ComputerGraph>,
) {
    for edge in graph.0.raw_edges() {
        let src_pos = graph.0[edge.source()].position;
        let dst_pos = graph.0[edge.target()].position;

        let diff = dst_pos - src_pos;
        let len = diff.length();
        let pos = (src_pos + dst_pos) / 2.0;
        let angle = diff.y.atan2(diff.x);
        const THICKNESS: f32 = 0.01;

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(len, THICKNESS))),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
            Transform::from_xyz(pos.x, pos.y, 0.0).with_rotation(Quat::from_rotation_z(angle)),
        ));
    }

    for node in graph.0.raw_nodes() {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(0.03))),
            MeshMaterial2d(materials.add(node.weight.color)),
            Transform::from_xyz(node.weight.position.x, node.weight.position.y, 0.0),
        ));
    }
}
