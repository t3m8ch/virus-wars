use bevy::{
    camera::ScalingMode, core_pipeline::tonemapping::Tonemapping, platform::collections::HashSet,
    post_process::bloom::Bloom, prelude::*,
};
use petgraph::graph::NodeIndex;

use crate::{
    components::{GameNode, Owner},
    resources::{ComputerGraph, FlowMap, GraphEntityMap, InteractionState},
    systems::{
        ai::ai_behavior,
        interaction::handle_interaction,
        packet::{move_packets, spawn_packets},
        visual::update_visuals,
    },
};

mod components;
mod resources;
mod systems;

const PACKET_SPEED: f32 = 1.0;
const NODE_MAX_HP: f32 = 100.0;
const PACKET_POWER: f32 = 1.0;
const SPAWN_INTERVAL: f32 = 0.1;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<InteractionState>()
        .init_resource::<GraphEntityMap>()
        .init_resource::<FlowMap>()
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                handle_interaction,
                ai_behavior,
                spawn_packets,
                move_packets,
                update_visuals,
            )
                .chain(),
        )
        .run();
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut entity_map: ResMut<GraphEntityMap>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 2.5,
            },
            ..OrthographicProjection::default_2d()
        }),
        Tonemapping::TonyMcMapface,
        Bloom::default(),
    ));

    let computer_graph = ComputerGraph::random();
    let graph = &computer_graph.0;

    let player_start_idx = NodeIndex::new(0);
    let enemy_start_idx = NodeIndex::new(graph.node_count() - 1);

    let mesh_circle = meshes.add(Circle::new(0.06));
    let mesh_edge = meshes.add(Rectangle::new(1.0, 0.02));

    for node_idx in graph.node_indices() {
        let node_data = graph[node_idx];

        let (owner, hp) = if node_idx == player_start_idx {
            (Owner::Player, 100.0)
        } else if node_idx == enemy_start_idx {
            (Owner::Enemy, 100.0)
        } else {
            (Owner::Neutral, 50.0)
        };

        let color = owner.color();
        let material = materials.add(ColorMaterial::from(color));

        let entity = commands
            .spawn((
                Mesh2d(mesh_circle.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(node_data.position.x, node_data.position.y, 1.0),
                GameNode {
                    index: node_idx,
                    hp,
                    owner,
                    targets: HashSet::new(),
                    timer: Timer::from_seconds(SPAWN_INTERVAL, TimerMode::Repeating),
                },
            ))
            .id();

        entity_map.nodes.insert(node_idx, entity);
    }

    let edge_color = materials.add(Color::srgb(0.2, 0.2, 0.2));

    for edge_idx in graph.edge_indices() {
        let (u, v) = graph.edge_endpoints(edge_idx).unwrap();
        let pos_a = graph[u].position;
        let pos_b = graph[v].position;

        let diff = pos_b - pos_a;
        let len = diff.length();
        let pos = (pos_a + pos_b) / 2.0;
        let angle = diff.y.atan2(diff.x);

        let entity = commands
            .spawn((
                Mesh2d(mesh_edge.clone()),
                MeshMaterial2d(edge_color.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.0)
                    .with_rotation(Quat::from_rotation_z(angle))
                    .with_scale(Vec3::new(len, 1.0, 1.0)),
            ))
            .id();

        entity_map.edges.insert(edge_idx, entity);
    }

    commands.insert_resource(computer_graph);
}
