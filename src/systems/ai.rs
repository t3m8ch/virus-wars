use bevy::ecs::system::{Query, Res};

use crate::{
    components::{GameNode, Owner},
    resources::ComputerGraph,
};

pub fn ai_behavior(nodes_q: Query<&mut GameNode>, graph_res: Res<ComputerGraph>) {
    let mut commands = Vec::new();

    for node in nodes_q.iter() {
        if node.owner == Owner::Enemy {
            for _ in graph_res.0.neighbors(node.index) {
                commands.push(node.index);
            }
        }
    }
}
