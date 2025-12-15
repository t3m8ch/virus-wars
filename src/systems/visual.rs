use bevy::{
    asset::Assets,
    color::{Color, LinearRgba, Mix},
    ecs::{
        query::Without,
        system::{Query, Res, ResMut},
    },
    input::{ButtonInput, keyboard::KeyCode},
    sprite_render::{ColorMaterial, MeshMaterial2d},
};

use crate::{
    NODE_MAX_HP,
    components::{GameNode, Packet},
    resources::{ComputerGraph, FlowMap, GraphEntityMap, InteractionState},
};

pub fn update_visuals(
    nodes_q: Query<(&GameNode, &MeshMaterial2d<ColorMaterial>)>,
    mut edges_q: Query<&mut MeshMaterial2d<ColorMaterial>, (Without<GameNode>, Without<Packet>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    interaction: Res<InteractionState>,
    graph_res: Res<ComputerGraph>,
    entity_map: Res<GraphEntityMap>,
    flow_map: Res<FlowMap>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let color_default_edge = materials.add(Color::srgb(0.2, 0.2, 0.2));
    let color_flow_edge = materials.add(Color::srgb(0.0, 0.5, 1.0));

    let is_erasing = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let path_color_value = if is_erasing {
        Color::srgb(1.0, 0.0, 0.0)
    } else {
        Color::srgb(1.0, 1.0, 0.0)
    };
    let color_path_edge = materials.add(path_color_value);

    for mut mat in edges_q.iter_mut() {
        mat.0 = color_default_edge.clone();
    }

    for (source, targets) in &flow_map.flows {
        for &target in targets {
            if let Some(edge_idx) = graph_res.0.find_edge(*source, target) {
                if let Some(&entity) = entity_map.edges.get(&edge_idx) {
                    if let Ok(mut mat) = edges_q.get_mut(entity) {
                        mat.0 = color_flow_edge.clone();
                    }
                }
            }
        }
    }

    if !interaction.path.is_empty() {
        for window in interaction.path.windows(2) {
            let u = window[0];
            let v = window[1];
            if let Some(edge_idx) = graph_res.0.find_edge(u, v) {
                if let Some(&entity) = entity_map.edges.get(&edge_idx) {
                    if let Ok(mut mat) = edges_q.get_mut(entity) {
                        mat.0 = color_path_edge.clone();
                    }
                }
            }
        }
    }

    for (node, mat_handle) in nodes_q.iter() {
        if let Some(material) = materials.get_mut(mat_handle) {
            let mut base_color = node.owner.color();

            if Some(node.index) == interaction.selected_source {
                base_color = Color::WHITE;
            } else if interaction.path.contains(&node.index) {
                let tint = if is_erasing {
                    Color::srgb(1.0, 0.0, 0.0)
                } else {
                    Color::srgb(1.0, 1.0, 0.0)
                };
                base_color = base_color.mix(&tint, 0.6);
            } else if Some(node.index) == interaction.hovered_node {
                base_color = base_color.mix(&Color::srgb(1.0, 1.0, 0.0), 0.3);
            }

            let hp_factor = 0.3 + 0.7 * (node.hp / NODE_MAX_HP);
            let final_color = LinearRgba::from(base_color);

            material.color = Color::LinearRgba(LinearRgba {
                red: final_color.red * hp_factor,
                green: final_color.green * hp_factor,
                blue: final_color.blue * hp_factor,
                alpha: 1.0,
            });
        }
    }
}
