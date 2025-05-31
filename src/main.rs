use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_seedling::prelude::*;
use bevy_simple_subsecond_system::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Wasm builds will check for meta files (that don't exist) if this isn't set.
            // This causes errors and even panics in web builds on itch.
            // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins((
            SeedlingPlugin::default(),
            HanabiPlugin,
            SimpleSubsecondPlugin::default(),
            PhysicsPlugins::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, greet)
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite {
        image: asset_server.load("ducky.png"),
        flip_y: true,
        ..Default::default()
    });
}

#[hot]
fn greet(time: Res<Time>) {
    info_once!("nah world from reload at {}", time.elapsed_secs());
}
