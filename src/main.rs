use bevy::{
    camera::ScalingMode,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    window::PrimaryWindow,
};
use petgraph::{
    Graph, Undirected,
    algo::astar,
    graph::{EdgeIndex, NodeIndex},
    visit::{Bfs, EdgeRef},
};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<InteractionState>()
        .init_resource::<GraphEntityMap>()
        .add_systems(Startup, (setup_computer_graph, draw_computer_graph).chain())
        .add_systems(Update, (handle_interaction, update_visuals).chain())
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

#[derive(Resource, Default)]
struct GraphEntityMap {
    nodes: HashMap<NodeIndex, Entity>,
    edges: HashMap<EdgeIndex, Entity>,
}

#[derive(Resource, Default)]
struct InteractionState {
    start_node: Option<NodeIndex>,
    hovered_node: Option<NodeIndex>,
    path: Vec<NodeIndex>,
}

#[derive(Clone, Copy)]
struct ComputerNode {
    position: Vec2,
    color: Color,
}

#[derive(Component)]
struct GraphMaterialHandles {
    normal: Handle<ColorMaterial>,
    highlight: Handle<ColorMaterial>,
    selected: Handle<ColorMaterial>,
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
    mut entity_map: ResMut<GraphEntityMap>,
) {
    // Подготавливаем материалы (цвета)
    let node_color_normal = materials.add(Color::WHITE);
    let node_color_selected = materials.add(Color::srgb(0.0, 1.0, 0.0)); // Зеленый (старт)
    let node_color_highlight = materials.add(Color::srgb(1.0, 1.0, 0.0)); // Желтый (путь)

    let edge_color_normal = materials.add(Color::srgb(1.0, 0.0, 0.0)); // Красный
    let edge_color_highlight = materials.add(Color::srgb(0.0, 0.8, 1.0)); // Голубой (путь)

    let mesh_circle = meshes.add(Circle::new(0.03));

    // Рисуем узлы
    for node_idx in graph.0.node_indices() {
        let node = graph.0[node_idx];
        let entity = commands
            .spawn((
                Mesh2d(mesh_circle.clone()),
                MeshMaterial2d(node_color_normal.clone()),
                Transform::from_xyz(node.position.x, node.position.y, 1.0), // Z=1 (поверх ребер)
                // Сохраняем handles материалов в компоненте, чтобы не искать их каждый кадр
                GraphMaterialHandles {
                    normal: node_color_normal.clone(),
                    highlight: node_color_highlight.clone(),
                    selected: node_color_selected.clone(),
                },
            ))
            .id();
        entity_map.nodes.insert(node_idx, entity);
    }

    // Рисуем ребра
    for edge_idx in graph.0.edge_indices() {
        let (u, v) = graph.0.edge_endpoints(edge_idx).unwrap();
        let pos_a = graph.0[u].position;
        let pos_b = graph.0[v].position;

        let diff = pos_b - pos_a;
        let len = diff.length();
        let pos = (pos_a + pos_b) / 2.0;
        let angle = diff.y.atan2(diff.x);

        // В Bevy 0.17+ Mesh2d заменил MaterialMesh2dBundle
        let rect_mesh = meshes.add(Rectangle::new(len, 0.01));

        let entity = commands
            .spawn((
                Mesh2d(rect_mesh),
                MeshMaterial2d(edge_color_normal.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.0).with_rotation(Quat::from_rotation_z(angle)),
                GraphMaterialHandles {
                    normal: edge_color_normal.clone(),
                    highlight: edge_color_highlight.clone(),
                    selected: edge_color_highlight.clone(),
                },
            ))
            .id();
        entity_map.edges.insert(edge_idx, entity);
    }
}

fn handle_interaction(
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<InteractionState>,
    graph: Res<ComputerGraph>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };
    let Ok(window) = window_q.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(ray) = camera.viewport_to_world(cam_transform, cursor_pos) else {
        return;
    };
    let world_pos = ray.origin.truncate();

    let mut closest_node = None;
    let mut min_dist = 0.05;

    for node_idx in graph.0.node_indices() {
        let node_pos = graph.0[node_idx].position;
        let dist = node_pos.distance(world_pos);
        if dist < min_dist {
            min_dist = dist;
            closest_node = Some(node_idx);
        }
    }

    state.hovered_node = closest_node;

    if buttons.just_pressed(MouseButton::Left) {
        if let Some(node) = closest_node {
            state.start_node = Some(node);
            state.path.clear();
        } else {
            state.start_node = None;
            state.path.clear();
        }
    }

    if let (Some(start), Some(end)) = (state.start_node, state.hovered_node) {
        if start != end {
            let path_result = astar(
                &graph.0,
                start,
                |finish| finish == end,
                |e| {
                    let (u, v) = graph.0.edge_endpoints(e.id()).unwrap();
                    graph.0[u].position.distance(graph.0[v].position)
                },
                |_| 0.0,
            );

            if let Some((_, path)) = path_result {
                state.path = path;
            } else {
                state.path.clear();
            }
        } else {
            state.path.clear();
        }
    }
}

fn update_visuals(
    state: Res<InteractionState>,
    graph: Res<ComputerGraph>,
    entity_map: Res<GraphEntityMap>,
    mut materials_q: Query<(&mut MeshMaterial2d<ColorMaterial>, &GraphMaterialHandles)>,
) {
    for (mut mat, handles) in materials_q.iter_mut() {
        mat.0 = handles.normal.clone();
    }

    if let Some(start) = state.start_node {
        if let Some(&entity) = entity_map.nodes.get(&start) {
            if let Ok((mut mat, handles)) = materials_q.get_mut(entity) {
                mat.0 = handles.selected.clone();
            }
        }
    }

    if !state.path.is_empty() {
        for &node_idx in &state.path {
            if Some(node_idx) == state.start_node {
                continue;
            }

            if let Some(&entity) = entity_map.nodes.get(&node_idx) {
                if let Ok((mut mat, handles)) = materials_q.get_mut(entity) {
                    mat.0 = handles.highlight.clone();
                }
            }
        }

        for window in state.path.windows(2) {
            let u = window[0];
            let v = window[1];

            if let Some(edge_idx) = graph.0.find_edge(u, v) {
                if let Some(&entity) = entity_map.edges.get(&edge_idx) {
                    if let Ok((mut mat, handles)) = materials_q.get_mut(entity) {
                        mat.0 = handles.highlight.clone();
                    }
                }
            }
        }
    }
}
