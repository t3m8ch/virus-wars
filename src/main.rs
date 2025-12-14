use bevy::{camera::ScalingMode, prelude::*};
use petgraph::{Graph, Undirected};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_computer_graph, draw_computer_graph).chain())
        .run();
}

#[derive(Resource)]
struct ComputerGraph(Graph<ComputerNode, (), Undirected>);

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

    let mut graph = Graph::new_undirected();
    let node1 = graph.add_node(ComputerNode::new(Vec2::new(-0.6, 0.8), Color::WHITE));
    let node2 = graph.add_node(ComputerNode::new(Vec2::new(0.7, 0.3), Color::WHITE));
    let node3 = graph.add_node(ComputerNode::new(Vec2::new(-0.2, -0.2), Color::WHITE));
    graph.add_edge(node1, node2, ());
    graph.add_edge(node1, node3, ());
    graph.add_edge(node2, node3, ());

    commands.insert_resource(ComputerGraph(graph));
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
        const THICKNESS: f32 = 0.02;

        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(len, THICKNESS))),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
            Transform::from_xyz(pos.x, pos.y, 0.0).with_rotation(Quat::from_rotation_z(angle)),
        ));
    }

    for node in graph.0.raw_nodes() {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(0.05))),
            MeshMaterial2d(materials.add(node.weight.color)),
            Transform::from_xyz(node.weight.position.x, node.weight.position.y, 0.0),
        ));
    }
}
