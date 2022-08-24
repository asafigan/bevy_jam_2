use battle::{BattlePlugin, BattlePrefab, EnemyKind, EnemyPrefab};
use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
};
use bevy_tweening::TweeningPlugin;
use board::{BoardPlugin, BoardPrefab, WorldCursor};
use prefab::spawn;
use utils::UtilsPlugin;

mod battle;
mod board;
mod prefab;
mod tween_untils;
mod utils;

pub fn build_app() -> App {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        present_mode: bevy::window::PresentMode::Immediate,
        ..Default::default()
    })
    .insert_resource(AmbientLight {
        brightness: 0.5,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(TweeningPlugin)
    .add_plugin(BoardPlugin)
    .add_plugin(UtilsPlugin)
    .add_plugin(BattlePlugin)
    .add_startup_system(setup);

    app
}

const BATTLE_LAYER: RenderLayers = RenderLayers::layer(0);
const BOARD_LAYER: RenderLayers = RenderLayers::layer(1);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(Camera3dBundle {
            projection: OrthographicProjection {
                scale: 3.0,
                scaling_mode: ScalingMode::FixedVertical(2.0),
                ..default()
            }
            .into(),
            transform: Transform::from_translation(Vec3::Z * 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::None,
                ..default()
            },
            camera: Camera {
                // renders after / on top of the main camera
                priority: 1,
                ..default()
            },
            ..default()
        })
        .insert(WorldCursor::default())
        .insert(BOARD_LAYER);

    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 4.0, 10.0)
                .looking_at([0.0, 0.0, 3.0].into(), Vec3::Y),
            ..default()
        })
        .insert(BATTLE_LAYER);

    spawn(
        BoardPrefab {
            layers: BOARD_LAYER,
            gems: BoardPrefab::random_gems(),
            transform: Transform::from_xyz(0.0, -1.0, 0.0).with_scale(Vec3::splat(0.5)),
        },
        &mut commands,
    );

    spawn(
        BattlePrefab {
            environment: asset_server.load("scenes/battles/super_basic.glb#Scene0"),
            layers: BATTLE_LAYER,
            enemy: EnemyPrefab {
                kind: EnemyKind::random(),
            },
        },
        &mut commands,
    );
}
