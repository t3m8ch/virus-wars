use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        entity::Entity,
        system::{Commands, Query, Res, ResMut},
    },
    math::primitives::Circle,
    mesh::{Mesh, Mesh2d},
    platform::collections::{HashMap, HashSet},
    sprite_render::{ColorMaterial, MeshMaterial2d},
    time::Time,
    transform::components::Transform,
};
use petgraph::graph::NodeIndex;

use crate::{
    NODE_MAX_HP, PACKET_POWER, PACKET_SPEED, SPAWN_INTERVAL,
    components::{GameNode, Owner, Packet},
    resources::{ComputerGraph, FlowMap, GraphEntityMap},
};

pub fn spawn_packets(
    mut commands: Commands,
    time: Res<Time>,
    mut nodes_q: Query<(&mut GameNode, &Transform)>,
    graph_res: Res<ComputerGraph>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    flow_map: Res<FlowMap>,
) {
    let node_states: HashMap<NodeIndex, (Owner, f32)> = nodes_q
        .iter()
        .map(|(n, _)| (n.index, (n.owner, n.hp)))
        .collect();

    let packet_mesh = meshes.add(Circle::new(0.015));

    for (mut node, transform) in nodes_q.iter_mut() {
        let mut active_targets = HashSet::new();

        if node.owner == Owner::Player {
            if let Some(targets) = flow_map.flows.get(&node.index) {
                for &t in targets {
                    active_targets.insert(t);
                }
            }
        } else if node.owner == Owner::Enemy {
            for neighbor_idx in graph_res.0.neighbors(node.index) {
                if let Some((neighbor_owner, neighbor_hp)) = node_states.get(&neighbor_idx) {
                    if *neighbor_owner != Owner::Enemy {
                        active_targets.insert(neighbor_idx);
                    } else if *neighbor_hp < NODE_MAX_HP {
                        active_targets.insert(neighbor_idx);
                    }
                }
            }
        }

        node.timer.tick(time.delta());

        if node.timer.just_finished() && !active_targets.is_empty() && node.owner != Owner::Neutral
        {
            let target_count = active_targets.len();
            let cooldown_mult = target_count as f32;

            node.timer.set_duration(std::time::Duration::from_secs_f32(
                SPAWN_INTERVAL * cooldown_mult,
            ));
            node.timer.reset();

            for &target_idx in &active_targets {
                let target_pos = graph_res.0[target_idx].position;
                let dist = transform.translation.truncate().distance(target_pos);

                let color = match node.owner {
                    Owner::Player => Color::srgb(0.5, 0.5, 1.0),
                    Owner::Enemy => Color::srgb(1.0, 0.5, 0.5),
                    _ => Color::WHITE,
                };

                commands.spawn((
                    Mesh2d(packet_mesh.clone()),
                    MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                    Transform::from_translation(transform.translation),
                    Packet {
                        from: node.index,
                        to: target_idx,
                        owner: node.owner,
                        progress: 0.0,
                        edge_len: dist,
                    },
                ));
            }
        }
    }
}

pub fn move_packets(
    mut commands: Commands,
    time: Res<Time>,
    mut packets_q: Query<(Entity, &mut Packet, &mut Transform)>,
    mut nodes_q: Query<&mut GameNode>,
    graph_res: Res<ComputerGraph>,
    entity_map: Res<GraphEntityMap>,
) {
    for (packet_entity, mut packet, mut transform) in packets_q.iter_mut() {
        let speed = PACKET_SPEED / packet.edge_len;
        packet.progress += speed * time.delta_secs();

        let start_pos = graph_res.0[packet.from].position;
        let end_pos = graph_res.0[packet.to].position;

        let current_pos = start_pos.lerp(end_pos, packet.progress);
        transform.translation.x = current_pos.x;
        transform.translation.y = current_pos.y;

        if packet.progress >= 1.0 {
            commands.entity(packet_entity).despawn();

            if let Some(&target_entity) = entity_map.nodes.get(&packet.to) {
                if let Ok(mut target_node) = nodes_q.get_mut(target_entity) {
                    process_hit(&mut target_node, packet.owner);
                }
            }
        }
    }
}

fn process_hit(node: &mut GameNode, packet_owner: Owner) {
    if node.owner == packet_owner {
        node.hp = (node.hp + PACKET_POWER).min(NODE_MAX_HP);
    } else {
        node.hp -= PACKET_POWER;
        if node.hp <= 0.0 {
            node.owner = packet_owner;
            node.hp = 10.0;
            node.targets.clear();
        }
    }
}
