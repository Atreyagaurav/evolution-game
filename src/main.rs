use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    sprite::MaterialMesh2dBundle,
};

const TIME_STEP: f32 = 1.0 / 60.0;
const TOP_BOUNDARY: f32 = 250.0;
const BOTTOM_BOUNDARY: f32 = -250.0;
const RIGHT_BOUNDARY: f32 = 250.0;
const LEFT_BOUNDARY: f32 = -250.0;
const BOUNDARY_THICKNESS: f32 = 4.0;

const BACKGROUND_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const BOUNDARY_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const FOOD_COLOR: Color = Color::rgb(0.1, 0.4, 0.1);
const ORGANISM_COLOR: Color = Color::rgb(0.1, 0.1, 0.4);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_plugin(HelloPlugin)
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Component)]
struct Organism;

#[derive(Component)]
struct Food;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Size(u8);

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Default)]
struct CollisionEvent;

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
    }
}

fn add_people(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());
    // Boundarys
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Left));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Right));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Bottom));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Top));

    // Organism
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::default().into()).into(),
            material: materials.add(ColorMaterial::from(ORGANISM_COLOR)),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
                .with_scale(Vec3::new(10.0, 10.0, 0.0)),
            ..default()
        },
        Organism,
        Size(5),
        Velocity(Vec2::new(5.0, 4.0)),
    ));
}

#[derive(Resource)]
struct GreetTimer(Timer);

fn greet_people(
    time: Res<Time>,
    mut timer: ResMut<GreetTimer>,
    query: Query<&Name, With<Organism>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for name in &query {
            println!("hello {}!", name.0);
        }
    }
}

// This bundle is a collection of the components that define a "boundary" in our game
#[derive(Bundle)]
struct BoundaryBundle {
    // You can nest bundles inside of other bundles like this
    // Allowing you to compose their functionality
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

/// Which side of the arena is this boundary located on?
enum BoundaryLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl BoundaryLocation {
    fn position(&self) -> Vec2 {
        match self {
            BoundaryLocation::Left => Vec2::new(LEFT_BOUNDARY, 0.),
            BoundaryLocation::Right => Vec2::new(RIGHT_BOUNDARY, 0.),
            BoundaryLocation::Bottom => Vec2::new(0., BOTTOM_BOUNDARY),
            BoundaryLocation::Top => Vec2::new(0., TOP_BOUNDARY),
        }
    }

    fn size(&self) -> Vec2 {
        let arena_height = TOP_BOUNDARY - BOTTOM_BOUNDARY;
        let arena_width = RIGHT_BOUNDARY - LEFT_BOUNDARY;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            BoundaryLocation::Left | BoundaryLocation::Right => {
                Vec2::new(BOUNDARY_THICKNESS, arena_height + BOUNDARY_THICKNESS)
            }
            BoundaryLocation::Bottom | BoundaryLocation::Top => {
                Vec2::new(arena_width + BOUNDARY_THICKNESS, BOUNDARY_THICKNESS)
            }
        }
    }
}

impl BoundaryBundle {
    // This "builder method" allows us to reuse logic across our boundary entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: BoundaryLocation) -> BoundaryBundle {
        BoundaryBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: location.position().extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surprising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: BOUNDARY_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
        }
    }
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GreetTimer(Timer::from_seconds(2.0, TimerMode::Repeating)))
            .add_startup_system(add_people)
            .add_system(apply_velocity);
    }
}
