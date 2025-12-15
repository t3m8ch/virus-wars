use bevy::{color::Color, ecs::component::Component, platform::collections::HashSet, time::Timer};
use petgraph::graph::NodeIndex;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    Neutral,
    Player,
    Enemy,
}

impl Owner {
    pub fn color(&self) -> Color {
        match self {
            Owner::Neutral => Color::srgb(1.5, 1.5, 1.5),
            Owner::Player => Color::srgb(0.0, 4.0, 5.0),
            Owner::Enemy => Color::srgb(5.0, 1.0, 1.0),
        }
    }
}

#[derive(Component)]
pub struct GameNode {
    pub index: NodeIndex,
    pub hp: f32,
    pub owner: Owner,
    pub targets: HashSet<NodeIndex>,
    pub timer: Timer,
}

#[derive(Component)]
pub struct Packet {
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub owner: Owner,
    pub progress: f32,
    pub edge_len: f32,
}
