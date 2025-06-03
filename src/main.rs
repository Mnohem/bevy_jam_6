use std::{
    f32::consts::{FRAC_PI_2, PI},
    sync::OnceLock,
};

use avian2d::prelude::*;
use bevy::{color::palettes::css, prelude::*, window::PrimaryWindow};
use bevy_hanabi::prelude::*;
use bevy_seedling::prelude::*;
use bevy_simple_subsecond_system::prelude::*;
use bevy_tnua::prelude::*;
use bevy_tnua_avian2d::*;

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
            // PhysicsDebugPlugin::default(),
            SeedlingPlugin::default(),
            HanabiPlugin,
            SimpleSubsecondPlugin::default(),
            PhysicsPlugins::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian2dPlugin::new(FixedUpdate),
        ))
        .insert_resource(WebbingVelocity(Vec2::splat(20.0)))
        .insert_resource(Gravity(Vec2::NEG_Y * 100.0))
        .add_event::<WebbingExtend>()
        .add_observer(observe_webbing)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                follow_mouse,
                shoot_webbing,
                movement_controls.in_set(TnuaUserControlsSystemSet),
            ),
        )
        .run()
}

const BOUNDARY_MEMBERSHIP: u32 = 0b1000;

const WEBBING_MEMBERSHIP: u32 = 0b10;
const WEBBING_FILTER: u32 = WEBBING_SENSOR_MEMBERSHIP | BOUNDARY_MEMBERSHIP; // | WEBBING_MEMBERSHIP;
//
const WEBBING_SENSOR_MEMBERSHIP: u32 = 0b100;
const WEBBING_SENSOR_FILTER: u32 = WEBBING_MEMBERSHIP;

const STICKY_SENSOR_MEMBERSHIP: u32 = 0b10000;
const STICKY_SENSOR_FILTER: u32 = u32::MAX & !PLAYER_MEMBERSHIP;

const PLAYER_MEMBERSHIP: u32 = 0b100000;
const PLAYER_FILTER: u32 = u32::MAX;

const WEBBING_HEIGHT: f32 = 5.0;
const WEBBING_WIDTH: f32 = 1.0;

/// Event for when a webbing leaves its origin sensor
#[derive(Event)]
struct WebbingExtend {
    origin: Vec2,
}

#[derive(Resource)]
struct WebbingVelocity(Vec2);

#[derive(Component, Clone)]
#[component(storage = "SparseSet")]
struct FreshWebbing;

#[derive(Component)]
struct Player;

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    cmd.spawn(Camera2d);
    // Floor
    let floor_height = 5.0;
    let floor_width = 200.0;
    cmd.spawn((
        Mesh2d(meshes.add(Rectangle::new(floor_width, floor_height))),
        MeshMaterial2d(materials.add(Color::from(css::YELLOW))),
        Transform::from_xyz(0.0, -50.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(floor_width, floor_height),
        CollisionLayers::new(BOUNDARY_MEMBERSHIP, LayerMask::ALL),
    ));

    let webbing_bundle = WEBBING_BUNDLE.get_or_init(|| {
        (
            Mesh2d(meshes.add(Rectangle::new(WEBBING_WIDTH, WEBBING_HEIGHT))),
            MeshMaterial2d(materials.add(Color::from(css::WHITE_SMOKE))),
            RigidBody::Dynamic,
            Collider::rectangle(WEBBING_WIDTH, WEBBING_HEIGHT),
            CollisionLayers::new(WEBBING_MEMBERSHIP, WEBBING_FILTER),
            FreshWebbing,
        )
    });

    let webbing_sensor = cmd
        .spawn((
            Collider::circle(10.0),
            Sensor,
            CollisionEventsEnabled,
            CollisionLayers::new(WEBBING_SENSOR_MEMBERSHIP, WEBBING_SENSOR_FILTER),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .observe(
            |mut t: Trigger<OnCollisionEnd>,
             mut cmd: Commands,
             fresh_q: Query<Entity, With<FreshWebbing>>,
             transform_q: Query<&GlobalTransform>| {
                t.propagate(false);
                if fresh_q.get(t.collider).is_ok()
                    && let Ok(transform) = transform_q.get(t.target())
                {
                    cmd.entity(t.collider).remove::<FreshWebbing>();
                    cmd.trigger_targets(
                        WebbingExtend {
                            origin: transform.translation().xy(),
                        },
                        t.collider,
                    );
                }
            },
        )
        .id();
    // Player
    let radius = 10.0;
    cmd.spawn((
        Mesh2d(meshes.add(Circle::new(radius))),
        MeshMaterial2d(materials.add(Color::from(css::DARK_BLUE))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Dynamic,
        Collider::circle(radius),
        CollisionLayers::new(PLAYER_MEMBERSHIP, PLAYER_FILTER),
        TnuaController::default(),
        TnuaAvian2dSensorShape(Collider::circle(radius - 0.1)),
        LockedAxes::ROTATION_LOCKED,
        Player,
    ))
    .add_child(webbing_sensor);
}

static WEBBING_BUNDLE: OnceLock<(
    Mesh2d,
    MeshMaterial2d<ColorMaterial>,
    RigidBody,
    Collider,
    CollisionLayers,
    FreshWebbing,
)> = OnceLock::new();
fn observe_webbing(
    t: Trigger<WebbingExtend>,
    mut cmd: Commands,
    vel: Res<WebbingVelocity>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if buttons.pressed(MouseButton::Left) {
        let connect_to = t.target();
        let connect_from = cmd
            .spawn((
                WEBBING_BUNDLE.get().unwrap().clone(),
                Transform::from_translation(t.origin.extend(0.0))
                    .with_rotation(Quat::from_axis_angle(Vec3::Z, vel.0.to_angle())),
                LinearVelocity::from(vel.0),
            ))
            // .observe(observe_webbing)
            .id();
        cmd.spawn(
            RevoluteJoint::new(connect_from, connect_to)
                .with_local_anchor_1(Vec2::NEG_Y * 4.0)
                .with_local_anchor_2(Vec2::Y * 4.0)
                .with_compliance(0.005),
        );
    }
}
#[hot]
fn shoot_webbing(
    mut cmd: Commands,
    vel: Res<WebbingVelocity>,
    buttons: Res<ButtonInput<MouseButton>>,
    ppos: Single<&Transform, With<Player>>,
    fresh_web: Query<Entity, With<FreshWebbing>>,
) {
    if buttons.pressed(MouseButton::Left) && fresh_web.single().is_err() {
        let sticky_sensor = cmd
            .spawn((
                Collider::rectangle(WEBBING_WIDTH + 1.0, WEBBING_HEIGHT + 1.0),
                Sensor,
                CollisionEventsEnabled,
                CollisionLayers::new(STICKY_SENSOR_MEMBERSHIP, STICKY_SENSOR_FILTER),
            ))
            .observe(
                |t: Trigger<OnCollisionStart>,
                 mut cmd: Commands,
                 parent_q: Query<&ChildOf>,
                 collisions: Collisions| {
                    let connect_from = parent_q.get(t.target()).unwrap().parent();
                    let connect_to = t.collider;
                    let contact_pair = collisions.get(t.target(), connect_to).unwrap();
                    let contact_point = contact_pair.find_deepest_contact().unwrap();
                    let (anchor1, anchor2) = if contact_pair.body1.unwrap() == connect_to {
                        (contact_point.local_point2, contact_point.local_point1)
                    } else {
                        (contact_point.local_point1, contact_point.local_point2)
                    };
                    cmd.spawn(
                        RevoluteJoint::new(connect_from, connect_to)
                            .with_local_anchor_1(anchor1)
                            .with_local_anchor_2(anchor2)
                            .with_compliance(0.005),
                    );
                    cmd.entity(t.target()).despawn();
                },
            )
            .id();
        cmd.spawn((
            WEBBING_BUNDLE.get().unwrap().clone(),
            Transform::from_translation(ppos.translation)
                .with_rotation(Quat::from_axis_angle(Vec3::Z, vel.0.to_angle() + PI)),
            LinearVelocity::from(vel.0),
        ))
        // .observe(observe_webbing)
        .add_child(sticky_sensor);
        // TODO child sensor for most things, with observer so collision creates a "sticking" joint
        // on parent
    }
}

fn movement_controls(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut TnuaController>) {
    let Ok(mut controller) = query.single_mut() else {
        return;
    };

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }

    // Feed the basis every frame. Even if the player doesn't move - just use `desired_velocity:
    // Vec3::ZERO`. `TnuaController` starts without a basis, which will make the character collider
    // just fall.
    controller.basis(TnuaBuiltinWalk {
        // The `desired_velocity` determines how the character will move.
        desired_velocity: direction.normalize_or_zero() * 100.0,
        // The `float_height` must be greater (even if by little) from the distance between the
        // character's center and the lowest point of its collider.
        float_height: 1.0,
        // `TnuaBuiltinWalk` has many other fields for customizing the movement - but they have
        // sensible defaults. Refer to the `TnuaBuiltinWalk`'s documentation to learn what they do.
        ..Default::default()
    });

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            // The height is the only mandatory field of the jump button.
            height: 4.0,
            // `TnuaBuiltinJump` also has customization fields with sensible defaults.
            ..Default::default()
        });
    }
}

#[derive(Component)]
struct FollowMouse;

#[hot]
fn follow_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    mut webbing_vel: ResMut<WebbingVelocity>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    player_pos: Single<&Transform, With<Player>>,
) -> Result {
    if buttons.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera.single()?;

        if let Some(cursor_world_pos) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
        {
            let mouse_delta =
                cursor_world_pos.extend(player_pos.translation.z) - player_pos.translation;
            let clamped_len = mouse_delta.xy().length().min(200.0);
            webbing_vel.0 = mouse_delta.xy().normalize_or_zero() * clamped_len;
        }
    }

    Ok(())
}
