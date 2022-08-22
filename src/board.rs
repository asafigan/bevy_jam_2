use crate::prefab::*;
use bevy::{
    asset::HandleId,
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::{
        shape::{Icosphere, RegularPolygon},
        *,
    },
    reflect::TypeUuid,
    render::camera::RenderTarget,
};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_state(BoardState::Waiting)
            .add_system_to_stage(CoreStage::PreUpdate, update_world_cursors)
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .after(update_world_cursors)
                    .with_system(update_tile_hover),
            )
            .add_system_set(
                SystemSet::on_update(BoardState::Waiting)
                    .with_system(hover_tile)
                    .with_system(unhover_tile)
                    .with_system(pickup_gem),
            )
            .add_system_set(
                SystemSet::on_update(BoardState::Moving)
                    .with_system(move_gem)
                    .with_system(drop_gem),
            )
            .add_system_set(SystemSet::on_exit(BoardState::Moving).with_system(return_gems));
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum BoardState {
    Waiting,
    Moving,
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

    meshes.set_untracked(
        TilePrefab::mesh_handle(),
        RegularPolygon {
            radius: f32::sqrt(0.5),
            sides: 4,
        }
        .into(),
    );
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

#[derive(Component)]
struct Hovered;

fn update_tile_hover(
    mut commands: Commands,
    tiles: Query<(Entity, &GlobalTransform), With<Tile>>,
    cursors: Query<&WorldCursor>,
    state: Res<State<BoardState>>,
) {
    if state.current() == &BoardState::Waiting {
        let cursor = cursors.single();

        for (entity, transform) in &tiles {
            let mut entity = commands.entity(entity);
            if let Some(position) = cursor.position {
                let matrix = transform.compute_matrix().inverse();
                let position = matrix.transform_point3(position.extend(0.0)).truncate();

                if position.max_element() < 0.5 && position.min_element() > -0.5 {
                    entity.insert(Hovered);
                } else {
                    entity.remove::<Hovered>();
                }
            } else {
                entity.remove::<Hovered>();
            }
        }
    }
}

fn hover_tile(
    mut materials: ResMut<Assets<StandardMaterial>>,
    tiles: Query<&Tile, Added<Hovered>>,
    mut gems: Query<(&Gem, &mut Handle<StandardMaterial>)>,
) {
    for tile in &tiles {
        let (gem, mut material) = gems.get_mut(tile.gem).unwrap();
        *material = materials.add(StandardMaterial {
            base_color: gem.element.color(),
            emissive: gem.element.color(),
            ..default()
        });
    }
}

fn unhover_tile(
    removed_hover: RemovedComponents<Hovered>,
    tiles: Query<&Tile>,
    mut gems: Query<(&Gem, &mut Handle<StandardMaterial>)>,
) {
    for tile in removed_hover.iter().filter_map(|x| tiles.get(x).ok()) {
        let (gem, mut material) = gems.get_mut(tile.gem).unwrap();
        *material = gem.element.material_handle();
    }
}

struct Moving(pub Entity);

fn pickup_gem(
    mut events: EventReader<MouseButtonInput>,
    hovered_tiles: Query<&Tile, With<Hovered>>,
    mut commands: Commands,
    mut state: ResMut<State<BoardState>>,
) {
    let start_pickup = events
        .iter()
        .filter(|e| e.button == MouseButton::Left)
        .fold(false, |_, current| current.state == ButtonState::Pressed);

    if start_pickup {
        if let Ok(tile) = hovered_tiles.get_single() {
            commands.insert_resource(Moving(tile.gem));
            state.replace(BoardState::Moving).unwrap();
        }
    }
}

fn drop_gem(mut events: EventReader<MouseButtonInput>, mut state: ResMut<State<BoardState>>) {
    let drop = events
        .iter()
        .filter(|e| e.button == MouseButton::Left)
        .fold(false, |_, current| current.state == ButtonState::Released);

    if drop {
        state.replace(BoardState::Waiting).unwrap();
    }
}

fn return_gems(
    mut gems: Query<&mut Transform, (Without<Tile>, With<Gem>)>,
    tiles: Query<(&Tile, &Transform)>,
) {
    for (tile, transform) in &tiles {
        let mut gem = gems.get_mut(tile.gem).unwrap();
        gem.translation = transform.translation.truncate().extend(1.0);
    }
}

fn move_gem(
    moving: Res<Moving>,
    mut gems: Query<&mut GlobalTransform, With<Gem>>,
    cursors: Query<&WorldCursor>,
) {
    if let Some(position) = cursors.single().position {
        let mut gem_transform = gems.get_mut(moving.0).unwrap();
        *gem_transform.translation_mut() = position.extend(2.0).into();
    }
}

#[derive(Debug, Clone, Copy)]
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

        rng.u8(..6).try_into().unwrap()
    }
}

impl TryFrom<u8> for Element {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == Element::Life as u8 => Ok(Element::Life),
            x if x == Element::Death as u8 => Ok(Element::Death),
            x if x == Element::Water as u8 => Ok(Element::Water),
            x if x == Element::Fire as u8 => Ok(Element::Fire),
            x if x == Element::Nature as u8 => Ok(Element::Nature),
            x if x == Element::Electric as u8 => Ok(Element::Electric),
            x => Err(x),
        }
    }
}

impl Element {
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

#[derive(Component)]
pub struct Gem {
    pub element: Element,
}

pub struct GemPrefab {
    pub element: Element,
    pub transform: Transform,
}

const GEM_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000);
const TILE_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 1);
const TILE_MATERIAL_ID: HandleId = HandleId::new(StandardMaterial::TYPE_UUID, 10_000 - 1);

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
        commands
            .entity(entity)
            .insert_bundle(PbrBundle {
                material: self.material_handle(),
                mesh: Self::mesh_handle(),
                transform: self.transform,
                ..default()
            })
            .insert(Gem {
                element: self.element,
            });
    }
}

pub struct BoardPrefab {
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

impl Prefab for BoardPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mut children = Vec::new();

        let middle = Vec3::new(6.0 / 2.0, 5.0 / 2.0, 0.0);

        for x in 0..6 {
            for y in 0..5 {
                let offset = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 1.0);
                let transform = Transform::from_translation(offset - middle);
                let gem = spawn(
                    GemPrefab {
                        element: self.gems[x][y],
                        transform: transform.with_scale(Vec3::splat(0.8))
                            * Transform::from_xyz(0.0, 0.0, 1.0),
                    },
                    commands,
                );

                let tile = TilePrefab { gem, transform };

                children.push(gem);
                children.push(spawn(tile, commands));
            }
        }

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .push_children(&children);
    }
}

#[derive(Component)]
pub struct Tile {
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
                gem: self.gem,
                mesh,
            })
            .add_child(mesh);
    }
}
