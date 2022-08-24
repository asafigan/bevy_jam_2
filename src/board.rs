use std::time::Duration;

use crate::prefab::*;
use crate::tween_untils::TweenType;
use crate::utils::{DelayedDespawn, DespawnEvent, DespawnReason};
use bevy::render::view::RenderLayers;
use bevy::{
    asset::HandleId,
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::{
        shape::{Icosphere, RegularPolygon},
        *,
    },
    reflect::TypeUuid,
    render::camera::RenderTarget,
    utils::HashSet,
};
use bevy_tweening::{
    lens::{TransformPositionLens, TransformScaleLens},
    Animator, Delay, EaseFunction, Tween, TweenCompleted, TweeningType,
};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter, EnumVariantNames};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TileEvent>()
            .add_event::<Match>()
            .add_event::<Fall>()
            .add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_system(change_gem_material)
            .add_state(BoardState::Waiting)
            .add_system_to_stage(CoreStage::PreUpdate, update_world_cursors)
            .add_system_to_stage(
                CoreStage::PreUpdate,
                track_tile_hover.after(update_world_cursors),
            )
            .add_system_set(SystemSet::on_enter(BoardState::Waiting).with_system(reset_timer))
            .add_system_set(SystemSet::on_update(BoardState::Waiting).with_system(pickup_gem))
            .add_system_set(
                SystemSet::on_update(BoardState::Swapping)
                    .with_system(move_gem)
                    .with_system(swap_gems)
                    .with_system(update_timer.after(swap_gems))
                    .with_system(drop_gem.after(update_timer)),
            )
            .add_system_set(SystemSet::on_exit(BoardState::Swapping).with_system(return_gems))
            .add_system_set(SystemSet::on_enter(BoardState::Matching).with_system(match_gems))
            .add_system_set(
                SystemSet::on_update(BoardState::Matching)
                    .with_system(destroy_matches)
                    .with_system(stop_matching),
            )
            .add_system_set(SystemSet::on_enter(BoardState::Falling).with_system(begin_fall))
            .add_system_set(
                SystemSet::on_update(BoardState::Falling)
                    .with_system(move_falling_gems)
                    .with_system(stop_falling),
            );
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum BoardState {
    Waiting,
    Swapping,
    Matching,
    Falling,
}

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>) {
    meshes.set_untracked(
        GemPrefab::mesh_handle(),
        Icosphere {
            radius: 0.5,
            subdivisions: 32,
        }
        .into(),
    );

    let square = RegularPolygon {
        radius: f32::sqrt(0.5),
        sides: 4,
    };

    meshes.set_untracked(TilePrefab::mesh_handle(), square.into());
    meshes.set_untracked(TimerPrefab::mesh_handle(), square.into());
}

fn add_materials(mut materials: ResMut<Assets<StandardMaterial>>) {
    for element in [
        Element::Life,
        Element::Death,
        Element::Water,
        Element::Fire,
        Element::Nature,
        Element::Electric,
    ] {
        materials.set_untracked(element.material_handle(), element.material())
    }

    materials.set_untracked(TilePrefab::material_handle(), default());
    materials.set_untracked(TimerPrefab::material_handle(), default());
}

#[derive(Component, Default)]
pub struct WorldCursor {
    pub position: Option<Vec2>,
}

fn update_world_cursors(
    windows: Res<Windows>,
    mut cameras: Query<(&Camera, &GlobalTransform, &mut WorldCursor)>,
) {
    for (camera, camera_transform, mut cursor) in &mut cameras {
        cursor.position = if let RenderTarget::Window(id) = camera.target {
            windows.get(id).and_then(|window| {
                let window_size = Vec2::new(window.width(), window.height());
                let cursor_position = window.cursor_position()?;

                // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
                let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;

                // matrix for undoing the projection and camera transform
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();

                // use it to convert ndc to world-space coordinates
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

                // reduce it to a 2D value
                Some(world_pos.truncate())
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct TileEvent {
    tile: Entity,
    info: TileEventInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TileEventInfo {
    Entered,
    Exited,
}

fn track_tile_hover(
    mut tiles: Query<(Entity, &mut Tile, &GlobalTransform)>,
    mut events: EventWriter<TileEvent>,
    cursors: Query<&WorldCursor>,
) {
    let cursor = cursors.single();

    for (entity, mut tile, transform) in &mut tiles {
        let mouse_in = if let Some(position) = cursor.position {
            let matrix = transform.compute_matrix().inverse();
            let position = matrix.transform_point3(position.extend(0.0)).truncate();

            position.max_element() < 0.5 && position.min_element() > -0.5
        } else {
            false
        };

        if tile.mouse_in != mouse_in {
            tile.mouse_in = mouse_in;
            events.send(TileEvent {
                tile: entity,
                info: if mouse_in {
                    TileEventInfo::Entered
                } else {
                    TileEventInfo::Exited
                },
            });
        }
    }
}

fn change_gem_material(
    mut updated: Local<HashSet<Entity>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<TileEvent>,
    tiles: Query<&Tile>,
    gems: Query<&Gem>,
    mut meshes: Query<&mut Handle<StandardMaterial>>,
    state: Res<State<BoardState>>,
) {
    // todo: bug: if the gem you grabbed is placed above matches and then falls it's material will not be reset
    // also the material needs to be reset during the match and fall state

    updated.extend(events.iter().map(|x| x.tile));

    if state.current() == &BoardState::Waiting {
        for entity in updated.drain() {
            let tile = tiles.get(entity).unwrap();
            let gem = gems.get(tile.gem).unwrap();
            let mut material = meshes.get_mut(gem.mesh).unwrap();
            *material = if tile.mouse_in {
                materials.add(StandardMaterial {
                    base_color: gem.element.color(),
                    emissive: gem.element.color(),
                    ..default()
                })
            } else {
                gem.element.material_handle()
            };
        }
    }
}

struct Swapping {
    swaps: u32,
    gem: Entity,
    current_tile: Entity,
    timer: Timer,
}

fn reset_timer(mut timers: Query<&mut Transform, With<TimerProgress>>) {
    for mut transform in &mut timers {
        transform.scale.x = 1.0;
    }
}

fn pickup_gem(
    mut events: EventReader<MouseButtonInput>,
    tiles: Query<(Entity, &Tile)>,
    mut commands: Commands,
    mut state: ResMut<State<BoardState>>,
) {
    let start_pickup = events
        .iter()
        .filter(|e| e.button == MouseButton::Left)
        .fold(false, |_, current| current.state == ButtonState::Pressed);

    if start_pickup {
        for (entity, tile) in &tiles {
            if tile.mouse_in {
                commands.insert_resource(Swapping {
                    swaps: 0,
                    gem: tile.gem,
                    current_tile: entity,
                    timer: Timer::from_seconds(9.0, false),
                });
                state.replace(BoardState::Swapping).unwrap();
            }
        }
    }
}

fn update_timer(
    mut swapping: ResMut<Swapping>,
    time: Res<Time>,
    mut timers: Query<&mut Transform, With<TimerProgress>>,
) {
    if swapping.swaps > 0 {
        swapping.timer.tick(time.delta());
    }

    for mut transform in &mut timers {
        transform.scale.x = swapping.timer.percent_left();
    }
}

fn swap_gems(
    mut swapping: ResMut<Swapping>,
    mut tiles: Query<(Entity, &mut Tile, &Transform), Without<Gem>>,
    mut gems: Query<&mut Transform, With<Gem>>,
    cursors: Query<&WorldCursor>,
    boards: Query<&GlobalTransform, With<Board>>,
) {
    // todo: bug: skip tile
    // todo: bug: diangles

    let cursor = cursors.single();

    if let Some(position) = cursor.position {
        let position = boards
            .single()
            .compute_matrix()
            .inverse()
            .transform_point3(position.extend(0.0))
            .truncate();

        let (closet, mut tile, _) = tiles
            .iter_mut()
            .max_by(|(_, _, a), (_, _, b)| {
                let a = a.translation.truncate().distance(position);
                let b = b.translation.truncate().distance(position);

                b.total_cmp(&a)
            })
            .unwrap();

        if closet != swapping.current_tile {
            let previous_gem = tile.gem;
            tile.gem = swapping.gem;

            let (_, mut tile, transform) = tiles.get_mut(swapping.current_tile).unwrap();

            tile.gem = previous_gem;

            let mut gem_transform = gems.get_mut(previous_gem).unwrap();

            gem_transform.translation = transform.translation;

            swapping.current_tile = closet;
            swapping.swaps += 1;
        }
    }
}

fn drop_gem(
    mut events: EventReader<MouseButtonInput>,
    mut state: ResMut<State<BoardState>>,
    swapping: Res<Swapping>,
) {
    let drop = events
        .iter()
        .filter(|e| e.button == MouseButton::Left)
        .fold(false, |_, current| current.state == ButtonState::Released);

    if drop || swapping.timer.finished() {
        state
            .replace(if swapping.swaps > 0 {
                BoardState::Matching
            } else {
                BoardState::Waiting
            })
            .unwrap();
    }
}

fn return_gems(
    mut gems: Query<&mut Transform, (Without<Tile>, With<Gem>)>,
    tiles: Query<&Transform, With<Tile>>,
    swapping: Res<Swapping>,
) {
    let transform = tiles.get(swapping.current_tile).unwrap();
    let mut gem = gems.get_mut(swapping.gem).unwrap();
    gem.translation = transform.translation;
}

fn move_gem(
    swapping: Res<Swapping>,
    mut gems: Query<(&mut Transform, &Parent), With<Gem>>,
    boards: Query<&GlobalTransform>,
    cursors: Query<&WorldCursor>,
) {
    if let Some(position) = cursors.single().position {
        let (mut gem_transform, parent) = gems.get_mut(swapping.gem).unwrap();
        let transform = boards.get(parent.get()).unwrap();
        let position = transform
            .compute_matrix()
            .inverse()
            .transform_point3(position.extend(0.0));

        gem_transform.translation = position.truncate().extend(1.0);
    }
}

#[derive(Debug)]
struct Match {
    tiles: HashSet<Entity>,
    element: Element,
}

#[derive(Clone, Copy)]
struct TileInfo {
    tile: Entity,
    element: Element,
}

fn match_gems(
    boards: Query<&Board>,
    tiles: Query<&Tile>,
    gems: Query<&Gem>,
    mut events: EventWriter<Match>,
) {
    // todo: combine adjacent matches

    let board = boards.single();

    let mut rows = vec![Vec::new(); 5];
    let mut columns = vec![Vec::new(); 6];

    for (x, column) in board.tiles.iter().enumerate() {
        for (y, &entity) in column.iter().enumerate() {
            let tile = tiles.get(entity).unwrap();
            let gem = gems.get(tile.gem).unwrap();
            let info = TileInfo {
                tile: entity,
                element: gem.element,
            };

            columns[x].push(info);
            rows[y].push(info);
        }
    }

    let mut matches = Vec::new();
    for row in rows.iter().chain(&columns) {
        let mut row = row.iter();
        let first = row.next().unwrap();
        let mut current_match = Match {
            tiles: [first.tile].into_iter().collect(),
            element: first.element,
        };

        for info in row {
            if current_match.element == info.element {
                current_match.tiles.insert(info.tile);
            } else {
                let previous = std::mem::replace(
                    &mut current_match,
                    Match {
                        tiles: [info.tile].into_iter().collect(),
                        element: info.element,
                    },
                );

                if previous.tiles.len() >= 3 {
                    matches.push(previous);
                };
            }
        }

        if current_match.tiles.len() >= 3 {
            matches.push(current_match);
        }
    }

    let mut index = 0;
    while index < matches.len() {
        let mut current = matches.remove(index);
        let mut i = 0;
        while i < matches.len() {
            if !matches[i].tiles.is_disjoint(&current.tiles) {
                let linked = matches.remove(i);
                current.tiles.extend(linked.tiles);
            } else {
                i += 1;
            }
        }

        matches.insert(index, current);
        index += 1;
    }

    events.send_batch(matches.into_iter());
}

fn destroy_matches(mut events: EventReader<Match>, tiles: Query<&Tile>, mut commands: Commands) {
    let start_delay = Duration::from_secs_f32(0.2);
    let delay_between_gems = Duration::from_secs_f32(0.0);
    let delay_between_matches = Duration::from_secs_f32(0.2);
    let animation_time = Duration::from_secs_f32(0.1);

    let mut delay = start_delay;

    for event in events.iter() {
        for &entity in &event.tiles {
            let tile = tiles.get(entity).unwrap();
            let tween = Tween::new(
                EaseFunction::BounceIn,
                TweeningType::Once,
                animation_time,
                TransformScaleLens {
                    start: Vec3::splat(1.0),
                    end: Vec3::splat(0.0),
                },
            );

            commands
                .entity(tile.gem)
                .insert(Animator::new(Delay::new(delay).then(tween)))
                .insert(
                    DelayedDespawn::new(delay + animation_time)
                        .with_reason(DespawnReason::DestroyGem),
                );

            delay += delay_between_gems;
        }

        delay += delay_between_matches;
    }
}

fn stop_matching(
    mut any_matches: Local<bool>,
    mut waiting_for: Local<usize>,
    mut events: EventReader<Match>,
    mut despawn_events: EventReader<DespawnEvent>,
    mut state: ResMut<State<BoardState>>,
) {
    if !events.is_empty() {
        *any_matches = true;
    }

    *waiting_for += events.iter().map(|e| e.tiles.len()).sum::<usize>();
    *waiting_for -= despawn_events
        .iter()
        .filter(|e| e.reason == Some(DespawnReason::DestroyGem))
        .count();

    if *waiting_for == 0 {
        state
            .replace(if *any_matches {
                BoardState::Falling
            } else {
                BoardState::Waiting
            })
            .unwrap();

        // needs to be reset or else any_matches will continue to be true
        // the next time BoardState::Matching is entered
        *any_matches = false;
    }
}

#[derive(Debug)]
struct Fall {
    tile: Entity,
    gem: FallingGem,
}

#[derive(Debug)]
enum FallingGem {
    Existing(Entity),
    New { height: u8 },
}

fn begin_fall(
    boards: Query<&Board>,
    tiles: Query<&Tile>,
    gems: Query<&Gem>,
    mut events: EventWriter<Fall>,
) {
    let board = boards.single();

    for column in &board.tiles {
        let mut height = 0;
        let mut num_stolen = 0;
        for (y, entity) in column.iter().enumerate() {
            let tile = tiles.get(*entity).unwrap();
            let missing = !gems.contains(tile.gem);
            let stolen = num_stolen > 0;

            if !missing && stolen {
                num_stolen -= 1;
            }

            if missing || stolen {
                let mut num_stolen_copy = num_stolen;
                let mut free_gems = column[(y + 1)..].iter().filter_map(|entity| {
                    let tile = tiles.get(*entity).unwrap();
                    let gem = gems.get(tile.gem).ok();
                    let missing = gem.is_none();
                    let stolen = num_stolen_copy > 0;

                    if !missing && stolen {
                        num_stolen_copy -= 1;
                    }

                    if stolen {
                        None
                    } else {
                        gem.map(|_| tile.gem)
                    }
                });

                events.send(Fall {
                    tile: *entity,
                    gem: if let Some(gem) = free_gems.next() {
                        num_stolen += 1;
                        FallingGem::Existing(gem)
                    } else {
                        height += 1;
                        FallingGem::New { height }
                    },
                })
            }
        }
    }
}

fn move_falling_gems(
    mut fall_events: EventReader<Fall>,
    mut tiles: Query<(&mut Tile, &Transform, &Parent)>,
    transforms: Query<&Transform>,
    mut commands: Commands,
) {
    for event in fall_events.iter() {
        let (mut tile, &transform, board) = tiles.get_mut(event.tile).unwrap();

        let (gem, start) = match event.gem {
            FallingGem::Existing(gem) => (gem, transforms.get(gem).unwrap().translation),
            FallingGem::New { height } => {
                let translation = Vec3::new(
                    transform.translation.x,
                    BOARD_MIDDLE.y - 0.5 + height as f32,
                    0.0,
                );
                (
                    spawn(
                        GemPrefab {
                            element: Element::random(),
                            transform: Transform::from_translation(translation),
                        },
                        &mut commands,
                    ),
                    translation,
                )
            }
        };

        let end = transform.translation;

        let height = start.y - end.y;
        let gravity = 30.0;
        let mut tween = Tween::new(
            EaseFunction::QuadraticIn,
            TweeningType::Once,
            Duration::from_secs_f32(f32::sqrt(2.0 * gravity * height) / gravity),
            TransformPositionLens { start, end },
        );

        tween.set_completed_event(TweenType::Fall.into());

        commands.entity(gem).insert(Animator::new(tween));

        commands.entity(**board).add_child(gem);

        tile.gem = gem;
    }
}

fn stop_falling(
    mut waiting_for: Local<usize>,
    mut fall_events: EventReader<Fall>,
    mut state: ResMut<State<BoardState>>,
    mut tween_events: EventReader<TweenCompleted>,
) {
    *waiting_for += fall_events.iter().count();

    *waiting_for -= tween_events
        .iter()
        .filter(|e| TweenType::try_from(e.user_data) == Ok(TweenType::Fall))
        .count();

    if *waiting_for == 0 {
        state.replace(BoardState::Matching).unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumVariantNames, EnumIter, EnumCount, Display)]
pub enum Element {
    Life,
    Death,
    Water,
    Fire,
    Nature,
    Electric,
}

impl Element {
    fn random() -> Element {
        let rng = fastrand::Rng::new();

        let n = rng.usize(..Self::COUNT);
        Self::iter().nth(n).unwrap()
    }

    fn material_handle(&self) -> Handle<StandardMaterial> {
        Handle::weak(HandleId::new(
            StandardMaterial::TYPE_UUID,
            10_000 + *self as u64,
        ))
    }

    fn material(&self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.color(),
            ..Default::default()
        }
    }

    fn color(&self) -> Color {
        match self {
            Element::Life => Color::PINK,
            Element::Death => Color::DARK_GRAY,
            Element::Water => Color::MIDNIGHT_BLUE,
            Element::Fire => Color::ORANGE_RED,
            Element::Nature => Color::DARK_GREEN,
            Element::Electric => Color::YELLOW,
        }
    }
}

const GEM_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000);

const TILE_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 1);
const TILE_MATERIAL_ID: HandleId = HandleId::new(StandardMaterial::TYPE_UUID, 10_000 - 1);

const TIMER_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 2);
const TIMER_MATERIAL_ID: HandleId = HandleId::new(StandardMaterial::TYPE_UUID, 10_000 - 2);

#[derive(Component)]
pub struct Gem {
    pub mesh: Entity,
    pub element: Element,
}

pub struct GemPrefab {
    pub element: Element,
    pub transform: Transform,
}

impl GemPrefab {
    fn material_handle(&self) -> Handle<StandardMaterial> {
        self.element.material_handle()
    }

    fn mesh_handle() -> Handle<Mesh> {
        Handle::weak(GEM_MESH_ID)
    }
}

impl Prefab for GemPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mesh = commands
            .spawn_bundle(PbrBundle {
                material: self.material_handle(),
                mesh: Self::mesh_handle(),
                transform: Transform::from_translation([0.0, 0.0, 1.0].into())
                    .with_scale(Vec3::splat(0.8)),
                ..default()
            })
            .id();

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(Gem {
                mesh,
                element: self.element,
            })
            .add_child(mesh);
    }
}

#[derive(Component)]
pub struct Board {
    tiles: [[Entity; 5]; 6],
}

pub struct BoardPrefab {
    pub layers: RenderLayers,
    pub gems: [[Element; 5]; 6],
    pub transform: Transform,
}

impl BoardPrefab {
    pub fn random_gems() -> [[Element; 5]; 6] {
        let mut gems = [[Element::Life; 5]; 6];
        for column in &mut gems {
            for gem in column {
                *gem = Element::random();
            }
        }

        gems
    }
}

const BOARD_MIDDLE: Vec3 = Vec3::new(6.0 / 2.0, 5.0 / 2.0, 0.0);

impl Prefab for BoardPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mut children = Vec::new();

        let middle = BOARD_MIDDLE;

        let mut tiles: Vec<[Entity; 5]> = Vec::new();
        for x in 0..6 {
            let mut column = Vec::new();
            for y in 0..5 {
                let offset = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);
                let transform = Transform::from_translation(offset - middle);
                let gem = spawn(
                    GemPrefab {
                        element: self.gems[x][y],
                        transform,
                    },
                    commands,
                );

                let tile = spawn(TilePrefab { gem, transform }, commands);

                children.push(gem);
                children.push(tile);

                column.push(tile);
            }

            tiles.push(column.try_into().unwrap());
        }

        children.push(spawn(
            TimerPrefab {
                transform: Transform::from_xyz(0.0, BOARD_MIDDLE.y + 0.5, 0.0)
                    .with_scale([BOARD_MIDDLE.x * 2.0, 0.25, 1.0].into()),
            },
            commands,
        ));

        let light = commands
            .spawn_bundle(DirectionalLightBundle {
                transform: Transform::from_rotation(
                    Quat::from_rotation_x(-45_f32.to_radians())
                        * Quat::from_rotation_y(-45_f32.to_radians()),
                ),
                ..default()
            })
            .id();

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(Board {
                tiles: tiles.try_into().unwrap(),
            })
            .insert(self.layers)
            .push_children(&children)
            .add_child(light);
    }
}

#[derive(Component)]
pub struct Tile {
    pub mouse_in: bool,
    pub gem: Entity,
    pub mesh: Entity,
}

struct TilePrefab {
    gem: Entity,
    transform: Transform,
}

impl TilePrefab {
    fn mesh_handle() -> Handle<Mesh> {
        Handle::weak(TILE_MESH_ID)
    }

    fn material_handle() -> Handle<StandardMaterial> {
        Handle::weak(TILE_MATERIAL_ID)
    }
}

impl Prefab for TilePrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mesh = commands
            .spawn_bundle(PbrBundle {
                mesh: Self::mesh_handle(),
                material: Self::material_handle(),
                transform: Transform::from_rotation(Quat::from_rotation_z(45_f32.to_radians())),
                ..default()
            })
            .id();

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(Tile {
                mouse_in: false,
                gem: self.gem,
                mesh,
            })
            .add_child(mesh);
    }
}

#[derive(Component)]
struct TimerProgress;

struct TimerPrefab {
    transform: Transform,
}

impl TimerPrefab {
    fn mesh_handle() -> Handle<Mesh> {
        Handle::weak(TIMER_MESH_ID)
    }

    fn material_handle() -> Handle<StandardMaterial> {
        Handle::weak(TIMER_MATERIAL_ID)
    }
}

impl Prefab for TimerPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mesh = commands
            .spawn_bundle(PbrBundle {
                mesh: Self::mesh_handle(),
                material: Self::material_handle(),
                transform: Transform::from_rotation(Quat::from_rotation_z(45_f32.to_radians())),
                ..default()
            })
            .id();

        let progress = commands
            .spawn_bundle(SpatialBundle::default())
            .insert(TimerProgress)
            .add_child(mesh)
            .id();

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .add_child(progress);
    }
}
