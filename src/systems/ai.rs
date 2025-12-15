use bevy::{
    ecs::system::{Query, Res, ResMut},
    time::Time,
};
use rand::seq::IndexedRandom;

use crate::{
    components::{GameNode, Owner},
    resources::{AiTimer, ComputerGraph},
};

pub fn ai_behavior(
    mut nodes_q: Query<&mut GameNode>,
    graph_res: Res<ComputerGraph>,
    time: Res<Time>,
    mut ai_timer: ResMut<AiTimer>,
) {
    ai_timer.0.tick(time.delta());
    if !ai_timer.0.is_finished() {
        return;
    }

    let mut rng = rand::rng();

    for mut node in nodes_q.iter_mut() {
        if node.owner == Owner::Enemy {
            node.targets.clear();

            if node.hp < 30.0 {
                continue;
            }

            let neighbors: Vec<_> = graph_res.0.neighbors(node.index).collect();

            if let Some(&target_idx) = neighbors.choose(&mut rng) {
                node.targets.insert(target_idx);
            }
        }
    }
}
