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
};
use rand::Rng;

// --- КОНФИГУРАЦИЯ БАЛАНСА ---
const PACKET_SPEED: f32 = 1.0;
const NODE_MAX_HP: f32 = 100.0;
const PACKET_POWER: f32 = 1.0; // Урон или лечение за один пакет
const SPAWN_INTERVAL: f32 = 0.1; // Секунды между выстрелами (кулдаун)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Owner {
    Neutral,
    Player,
    Enemy,
}

impl Owner {
    fn color(&self) -> Color {
        match self {
            Owner::Neutral => Color::srgb(0.5, 0.5, 0.5), // Серый
            Owner::Player => Color::srgb(0.0, 0.8, 1.0),  // Неоновый синий
            Owner::Enemy => Color::srgb(1.0, 0.2, 0.2),   // Красный
        }
    }
}

// --- КОМПОНЕНТЫ ---

#[derive(Component)]
struct GameNode {
    index: NodeIndex,
    hp: f32,
    owner: Owner,
    /// Список соседей, в которые мы сейчас хотим стрелять
    targets: HashSet<NodeIndex>,
    /// Таймер для стрельбы
    timer: Timer,
}

#[derive(Component)]
struct Packet {
    from: NodeIndex,
    to: NodeIndex,
    owner: Owner,
    progress: f32, // от 0.0 до 1.0 (процент пути)
    edge_len: f32, // Длина ребра для расчета скорости
}

// Ресурс графа (геометрия)
#[derive(Resource)]
struct ComputerGraph(Graph<ComputerNode, (), Undirected>);

#[derive(Clone, Copy)]
struct ComputerNode {
    position: Vec2,
}

#[derive(Resource, Default)]
struct GraphEntityMap {
    nodes: HashMap<NodeIndex, Entity>,
    edges: HashMap<EdgeIndex, Entity>,
}

#[derive(Resource, Default)]
struct InteractionState {
    selected_source: Option<NodeIndex>,
    hovered_node: Option<NodeIndex>,
    path: Vec<NodeIndex>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<InteractionState>()
        .init_resource::<GraphEntityMap>()
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                handle_interaction, // Ввод игрока
                ai_behavior,        // Логика врага
                spawn_packets,      // Генерация пакетов узлами
                move_packets,       // Движение пакетов
                update_visuals,     // Обновление цветов и UI
            )
                .chain(),
        )
        .run();
}

impl ComputerGraph {
    fn random() -> Self {
        const NODE_COUNT: usize = 30;
        const ATTEMPTS: usize = 20;
        const MIN_DIST: f32 = 0.2;
        const CONNECT_DIST: f32 = 0.45;

        let mut graph = Graph::new_undirected();
        let mut rng = rand::rng();

        let mut positions: Vec<Vec2> = Vec::with_capacity(NODE_COUNT);
        'outer: for _ in 0..(NODE_COUNT * ATTEMPTS) {
            if positions.len() >= NODE_COUNT {
                break;
            }
            // Чуть уменьшил границы, чтобы интерфейс не перекрывал
            let candidate = Vec2::new(rng.random_range(-0.8..0.8), rng.random_range(-0.8..0.8));

            for pos in &positions {
                if pos.distance(candidate) < MIN_DIST {
                    continue 'outer;
                }
            }
            positions.push(candidate);
        }

        let node_indices: Vec<NodeIndex> = positions
            .iter()
            .map(|&pos| graph.add_node(ComputerNode { position: pos }))
            .collect();

        // Соединяем близкие
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

        // Гарантируем связность (как в вашем коде)
        loop {
            let mut components: Vec<Vec<NodeIndex>> = Vec::new();
            let mut visited = HashSet::new();

            for &node in &node_indices {
                if !visited.contains(&node) {
                    let mut component = Vec::new();
                    let mut bfs = petgraph::visit::Bfs::new(&graph, node);
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

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut entity_map: ResMut<GraphEntityMap>,
) {
    // Камера
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 2.5,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Генерируем граф
    let computer_graph = ComputerGraph::random();
    let graph = &computer_graph.0;

    // Определяем стартовые позиции
    // 0 - Игрок, Последний - Враг (для простоты)
    let player_start_idx = NodeIndex::new(0);
    let enemy_start_idx = NodeIndex::new(graph.node_count() - 1);

    let mesh_circle = meshes.add(Circle::new(0.06)); // Чуть больше узлы
    let mesh_edge = meshes.add(Rectangle::new(1.0, 0.02)); // Пропорции потом изменим трансформом

    // Спавним узлы
    for node_idx in graph.node_indices() {
        let node_data = graph[node_idx];

        // Начальное владение
        let (owner, hp) = if node_idx == player_start_idx {
            (Owner::Player, 100.0)
        } else if node_idx == enemy_start_idx {
            (Owner::Enemy, 100.0)
        } else {
            (Owner::Neutral, 50.0) // Нейтралам дадим 50 HP
        };

        let color = owner.color();
        let material = materials.add(ColorMaterial::from(color));

        let entity = commands
            .spawn((
                Mesh2d(mesh_circle.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(node_data.position.x, node_data.position.y, 1.0),
                // Логический компонент узла
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

    // Спавним ребра (чисто визуал + связь)
    let edge_color = materials.add(Color::srgb(0.2, 0.2, 0.2)); // Темно-серый по умолчанию

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

// 4. Управление (Игрок)
fn handle_interaction(
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<InteractionState>,
    buttons: Res<ButtonInput<MouseButton>>,
    graph_res: Res<ComputerGraph>,
    mut nodes_q: Query<&mut GameNode>, // Читаем и пишем в узлы
    entity_map: Res<GraphEntityMap>,
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

    // Hit testing (поиск ближайшего узла под курсором)
    let mut hovered = None;
    let mut min_dist = 0.1; // Радиус клика

    for node_idx in graph_res.0.node_indices() {
        let pos = graph_res.0[node_idx].position;
        let dist = pos.distance(world_pos);
        if dist < min_dist {
            min_dist = dist;
            hovered = Some(node_idx);
        }
    }
    state.hovered_node = hovered;

    // ЛКМ: Выбор своего узла (источник)
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(idx) = hovered {
            // Проверяем, что это узел игрока
            if let Some(&entity) = entity_map.nodes.get(&idx) {
                if let Ok(node) = nodes_q.get(entity) {
                    if node.owner == Owner::Player {
                        state.selected_source = Some(idx);
                        println!("Source selected: {:?}", idx);
                    }
                }
            }
        } else {
            state.selected_source = None; // Клик в пустоту - сброс
        }
    }

    state.path.clear(); // Сбрасываем старый путь
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

    // ПКМ: Выбор цели (Target) и построение маршрута
    if buttons.just_pressed(MouseButton::Right) {
        if let (Some(source_idx), Some(target_idx)) = (state.selected_source, hovered) {
            if source_idx == target_idx {
                return;
            }

            // Строим путь BFS/A*
            let path_result = astar(
                &graph_res.0,
                source_idx,
                |finish| finish == target_idx,
                |_| 1.0, // Вес ребра 1, ищем кратчайший по хопам
                |_| 0.0,
            );

            if let Some((_, path)) = path_result {
                // Логика "потока":
                // Мы идем по пути от source.
                // Source должен стрелять в path[1].
                // Если path[1] захвачен нами, он должен стрелять в path[2] и т.д.
                // В данной простой реализации мы просто добавляем задание "Стрелять" для source в path[1].
                // А логика "автоматической маршрутизации" будет работать глобально:
                // Узлы сами решают, куда стрелять, если у них есть "глобальная цель".
                // НО, следуя вашему ТЗ п.4: "Узел A начинает стрелять... в B. Как только B захвачен... начинает стрелять в C".

                // Упростим: Мы просто говорим Source: "Атакуй соседа, который ведет к Target".
                // А чтобы цепочка работала, мы просто будем передавать "Целеуказание" по цепочке.
                // Но для начала реализуем прямую команду: Source -> Next hop.

                if path.len() >= 2 {
                    let next_hop = path[1];

                    if let Some(&entity) = entity_map.nodes.get(&source_idx) {
                        if let Ok(mut node) = nodes_q.get_mut(entity) {
                            // Тоггл: если уже стреляем туда - перестаем, иначе начинаем
                            if node.targets.contains(&next_hop) {
                                node.targets.remove(&next_hop);
                            } else {
                                node.targets.insert(next_hop);
                            }
                        }
                    }
                }
            }
        }
    }
}

// 6. Противник (AI) - Жадный Рой
fn ai_behavior(nodes_q: Query<&mut GameNode>, graph_res: Res<ComputerGraph>) {
    // Враг пересчитывает логику не каждый кадр, но для простоты здесь сделаем каждый.
    // Итерируемся по всем узлам
    // Нельзя одновременно итерироваться мутабельно и читать граф соседей внутри запроса без unsafe или сбора данных.
    // Соберем данные для AI команд.

    let mut commands = Vec::new();

    for node in nodes_q.iter() {
        if node.owner == Owner::Enemy {
            // Смотрим соседей
            for _ in graph_res.0.neighbors(node.index) {
                // Нам нужно узнать владельца соседа. Это сложно в одном query.
                // Придется сделать поиск владельца соседа.
                // Оптимизация: хранить владельца в NodeState, а доступ через EntityMap медленный?
                // Сделаем "хак": пройдемся по nodes_q второй раз внутри нельзя.
                // Поэтому разобьем на два прохода: чтение состояний -> принятие решений.
                commands.push(node.index);
            }
        }
    }

    // Это место неоптимально для реального ECS, но для Bevy и малого графа сойдет:
    // Мы не можем легко получить компонент соседа, зная его NodeIndex, без мапы.
    // Но у нас есть логика "Жадный рой":
    // "Если у красного узла есть нейтральный или вражеский сосед — он начинает спамить в него".

    // В Bevy проще сделать так: AI просто "включает" стрельбу во ВСЕХ соседей, которые не Красные и не полные HP.
}

// Улучшенная версия AI (без сложных запросов)
// AI просто всегда атакует всех соседей, которые не его цвета.
// И лечит своих, если они ранены.
fn spawn_packets(
    mut commands: Commands,
    time: Res<Time>,
    mut nodes_q: Query<(&mut GameNode, &Transform)>,
    graph_res: Res<ComputerGraph>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Получим состояние всех узлов в мапу для быстрого доступа (кто чей)
    let node_states: HashMap<NodeIndex, (Owner, f32)> = nodes_q
        .iter()
        .map(|(n, _)| (n.index, (n.owner, n.hp)))
        .collect();

    let packet_mesh = meshes.add(Circle::new(0.015));

    for (mut node, transform) in nodes_q.iter_mut() {
        // AI Logic inject: Если это Враг, он автоматически обновляет свои targets
        if node.owner == Owner::Enemy {
            node.targets.clear();
            for neighbor_idx in graph_res.0.neighbors(node.index) {
                if let Some((neighbor_owner, neighbor_hp)) = node_states.get(&neighbor_idx) {
                    // Атака: если сосед не враг
                    if *neighbor_owner != Owner::Enemy {
                        node.targets.insert(neighbor_idx);
                    }
                    // Лечение: если сосед враг, но ранен
                    else if *neighbor_hp < NODE_MAX_HP {
                        node.targets.insert(neighbor_idx);
                    }
                }
            }
        }

        // Логика стрельбы (общая для всех)
        node.timer.tick(time.delta());
        if node.timer.just_finished() && !node.targets.is_empty() && node.owner != Owner::Neutral {
            let target_count = node.targets.len();

            // Если целей много - стреляем во всех, но реже? Или просто спавним сразу кучу?
            // По ТЗ: "скорость потока к каждому падает в 2 раза".
            // Реализуем это через пропуск тактов или уменьшение таймера?
            // Проще: спавним пакеты всегда, но их сила или скорость зависит от кол-ва?
            // Давайте сделаем честно по таймеру: таймер сработал -> выпускаем пакеты.
            // Но таймер надо замедлять?
            // Сделаем так: таймер фиксированный. При срабатывании мы итерируемся по целям.
            // Но чтобы соблюсти баланс "деления потока", мы можем просто уменьшать таймер реже,
            // если целей много? Нет, лучше пусть стреляет во всех, но это стоит "ресурса"?
            // По ТЗ проще: "Скорость потока падает". Значит кулдаун для КОНКРЕТНОГО направления увеличивается.
            // В текущей реализации таймер один на узел.
            // Просто выпустим пакеты во все цели.

            for &target_idx in &node.targets {
                // Найдем позицию цели
                // (В реальном проекте лучше кэшировать)
                let target_pos = graph_res.0[target_idx].position;
                let my_pos = transform.translation.truncate();
                let dist = my_pos.distance(target_pos);

                let color = match node.owner {
                    Owner::Player => Color::srgb(0.5, 0.5, 1.0), // Светло-синий пакет
                    Owner::Enemy => Color::srgb(1.0, 0.5, 0.5),  // Розовый пакет
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

            // Сброс таймера с учетом штрафа за количество целей?
            // Если целей 2, то следующий выстрел должен быть через X * 2 времени?
            // Да, давайте сделаем это.
            let cooldown_mult = target_count as f32;
            node.timer.set_duration(std::time::Duration::from_secs_f32(
                SPAWN_INTERVAL * cooldown_mult,
            ));
            node.timer.reset();
        }
    }
}

// 3. Физика пакетов и коллизии
fn move_packets(
    mut commands: Commands,
    time: Res<Time>,
    mut packets_q: Query<(Entity, &mut Packet, &mut Transform)>,
    mut nodes_q: Query<&mut GameNode>,
    graph_res: Res<ComputerGraph>,
    entity_map: Res<GraphEntityMap>,
) {
    for (packet_entity, mut packet, mut transform) in packets_q.iter_mut() {
        // Движение
        let speed = PACKET_SPEED / packet.edge_len; // Нормализуем скорость, чтобы была const м/с
        packet.progress += speed * time.delta_secs();

        let start_pos = graph_res.0[packet.from].position;
        let end_pos = graph_res.0[packet.to].position;

        let current_pos = start_pos.lerp(end_pos, packet.progress);
        transform.translation.x = current_pos.x;
        transform.translation.y = current_pos.y;

        // Попадание
        if packet.progress >= 1.0 {
            // Удаляем пакет
            commands.entity(packet_entity).despawn();

            // Наносим эффект узлу
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
        // Лечение
        node.hp = (node.hp + PACKET_POWER).min(NODE_MAX_HP);
    } else {
        // Урон
        node.hp -= PACKET_POWER;
        if node.hp <= 0.0 {
            // Захват!
            node.owner = packet_owner;
            node.hp = 10.0; // Стартовое HP после захвата
            node.targets.clear(); // Сбрасываем старые приказы
            // Здесь можно добавить авто-продолжение пути, если реализовывать "Потоки" полностью
        }
    }
}

fn update_visuals(
    nodes_q: Query<(&GameNode, &MeshMaterial2d<ColorMaterial>)>,
    // ИСПРАВЛЕНИЕ: Добавили Without<Packet>, чтобы не перекрашивать снаряды в цвет ребер
    mut edges_q: Query<&mut MeshMaterial2d<ColorMaterial>, (Without<GameNode>, Without<Packet>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    interaction: Res<InteractionState>,
    graph_res: Res<ComputerGraph>,
    entity_map: Res<GraphEntityMap>,
) {
    // 1. Сброс цветов РЕБЕР в дефолтный (серый)
    let color_default_edge = materials.add(Color::srgb(0.2, 0.2, 0.2));
    let color_path_edge = materials.add(Color::srgb(1.0, 1.0, 0.0)); // Желтый путь

    for mut mat in edges_q.iter_mut() {
        mat.0 = color_default_edge.clone();
    }

    // 2. Подсветка РЕБЕР пути
    if !interaction.path.is_empty() {
        for window in interaction.path.windows(2) {
            let u = window[0];
            let v = window[1];
            if let Some(edge_idx) = graph_res.0.find_edge(u, v) {
                if let Some(&entity) = entity_map.edges.get(&edge_idx) {
                    // Теперь это безопасно меняет только ребра
                    if let Ok(mut mat) = edges_q.get_mut(entity) {
                        mat.0 = color_path_edge.clone();
                    }
                }
            }
        }
    }

    // 3. Обновление цветов УЗЛОВ
    for (node, mat_handle) in nodes_q.iter() {
        if let Some(material) = materials.get_mut(mat_handle) {
            let mut base_color = node.owner.color();

            if Some(node.index) == interaction.selected_source {
                base_color = Color::WHITE;
            } else if interaction.path.contains(&node.index) {
                base_color = base_color.mix(&Color::srgb(1.0, 1.0, 0.0), 0.6);
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
