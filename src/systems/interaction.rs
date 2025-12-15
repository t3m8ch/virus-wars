use bevy::{
    camera::Camera,
    ecs::{
        query::With,
        system::{Query, Res, ResMut},
    },
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window},
};
use petgraph::algo::astar;

use crate::{
    components::{GameNode, Owner},
    resources::{ComputerGraph, FlowMap, GraphEntityMap, InteractionState},
};

pub fn handle_interaction(
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<InteractionState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    graph_res: Res<ComputerGraph>,
    nodes_q: Query<&mut GameNode>,
    entity_map: Res<GraphEntityMap>,
    mut flow_map: ResMut<FlowMap>,
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

    let mut hovered = None;
    let mut min_dist = 0.1;

    for node_idx in graph_res.0.node_indices() {
        let pos = graph_res.0[node_idx].position;
        let dist = pos.distance(world_pos);
        if dist < min_dist {
            min_dist = dist;
            hovered = Some(node_idx);
        }
    }
    state.hovered_node = hovered;

    if mouse_buttons.just_pressed(MouseButton::Left) {
        if let Some(idx) = hovered {
            if let Some(&entity) = entity_map.nodes.get(&idx) {
                if let Ok(node) = nodes_q.get(entity) {
                    if node.owner == Owner::Player {
                        state.selected_source = Some(idx);
                        println!("Source selected: {:?}", idx);
                    }
                }
            }
        } else {
            state.selected_source = None;
        }
    }

    state.path.clear();
    if let (Some(source), Some(target)) = (state.selected_source, state.hovered_node) {
        if source != target {
            let path_result = astar(
                &graph_res.0,
                source,
                |finish| finish == target,
                |_| 1.0,
                |_| 0.0,
            );
            if let Some((_, path)) = path_result {
                state.path = path;
            }
        }
    }

    if mouse_buttons.just_pressed(MouseButton::Right) {
        if !state.path.is_empty() {
            let is_erasing =
                keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

            for window in state.path.windows(2) {
                let current_node = window[0];
                let next_node = window[1];

                if is_erasing {
                    if let Some(targets) = flow_map.flows.get_mut(&current_node) {
                        targets.remove(&next_node);
                        if targets.is_empty() {
                            flow_map.flows.remove(&current_node);
                        }
                    }
                } else {
                    let entry = flow_map.flows.entry(current_node).or_default();
                    entry.insert(next_node);
                }
            }

            if is_erasing {
                println!("Flows removed along path!");
            } else {
                println!("Flows added along path!");
            }
        }
    }
}
