use crate::prefab::*;
use bevy::{
    asset::HandleId,
    prelude::{
        shape::{Icosphere, RegularPolygon},
        *,
    },
    reflect::TypeUuid,
};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_system(hover_tile);
    }
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

fn hover_tile(
    mut materials: ResMut<Assets<StandardMaterial>>,
    windows: Res<Windows>,
    mut tiles: Query<(&Tile, &GlobalTransform)>,
    mut gems: Query<(&Gem, &mut Handle<StandardMaterial>)>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    let window = windows.primary();
    let (camera, camera_transform) = cameras.single();
    if let Some(mut cursor_position) = window.cursor_position() {
        let window_size = Vec2::new(window.width(), window.height());

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        cursor_position = world_pos.truncate();

        for (tile, transform) in &mut tiles {
            let matrix = transform.compute_matrix().inverse();

            let position = matrix
                .transform_point3(cursor_position.extend(0.0))
                .truncate();

            let (gem, mut material) = gems.get_mut(tile.gem).unwrap();

            if position.max_element() < 0.5 && position.min_element() > -0.5 {
                *material = materials.add(StandardMaterial {
                    base_color: gem.element.color(),
                    emissive: gem.element.color(),
                    ..default()
                });
            } else {
                *material = gem.element.material_handle();
            }
        }
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
