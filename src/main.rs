use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    sprite::MaterialMesh2dBundle,
};

use rand;

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

const ORGANISM_SIZE: Vec3 = Vec3::new(20.0, 20.0, 0.0);
const FOOD_SIZE: Vec3 = Vec3::new(10.0, 10.0, 0.0);

const ORGANISM_VELOCITY: Vec2 = Vec2::new(100.0, 100.0);

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
struct Energy(f32);

#[derive(Component)]
struct Name(String);

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Default)]
struct CollisionEvent;

fn random_xy() -> Vec3 {
    let (x, y): (f32, f32) = (rand::random(), rand::random());
    Vec3::new(
        (x - 0.5) * 2.0 * TOP_BOUNDARY,
        (y - 0.5) * 2.0 * RIGHT_BOUNDARY,
        0.0,
    )
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
    }
}

fn startup(
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
            transform: Transform::from_translation(random_xy()).with_scale(ORGANISM_SIZE),
            ..default()
        },
        Organism,
        Energy(1.0),
        Velocity(ORGANISM_VELOCITY),
    ));
}

#[derive(Resource)]
struct FoodTimer(Timer);

fn generate_food(
    time: Res<Time>,
    mut timer: ResMut<FoodTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::default().into()).into(),
                material: materials.add(ColorMaterial::from(FOOD_COLOR)),
                transform: Transform::from_translation(random_xy()).with_scale(FOOD_SIZE),
                ..default()
            },
            Food,
            Energy(0.1),
            Collider,
        ));
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

fn grow_organism(
    mut commands: Commands,
    mut organism_query: Query<(&mut Transform, &mut Energy), With<Organism>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (mut organism_transform, mut organism_energy) in &mut organism_query {
        if organism_energy.0 > 2.0 {
            organism_energy.0 = 1.0;
            // Multiple organism fails, may need to modify other queries
            // commands.spawn((
            //     MaterialMesh2dBundle {
            //         mesh: meshes.add(shape::Circle::default().into()).into(),
            //         material: materials.add(ColorMaterial::from(ORGANISM_COLOR)),
            //         transform: Transform::from_translation(random_xy()).with_scale(ORGANISM_SIZE),
            //         ..default()
            //     },
            //     Organism,
            //     Energy(1.0),
            //     Velocity(ORGANISM_VELOCITY),
            // ));
        }
        organism_transform.scale = ORGANISM_SIZE * organism_energy.0;
    }
}

fn check_for_collisions(
    mut commands: Commands,
    mut organism_query: Query<(&mut Velocity, &Transform, &mut Energy), With<Organism>>,
    collider_query: Query<(Entity, &Transform, Option<&Food>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let (mut organism_velocity, organism_transform, mut organism_energy) =
        organism_query.single_mut();
    let organism_size = organism_transform.scale.truncate();

    // check collision with walls
    for (collider_entity, transform, maybe_food) in &collider_query {
        let collision = collide(
            organism_transform.translation,
            organism_size,
            transform.translation,
            transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            // Sends a collision event so that other systems can react to the collision
            collision_events.send_default();

            // Foods should be despawned and increment the scoreboard on collision
            if maybe_food.is_some() {
                organism_energy.0 += 0.2;
                commands.entity(collider_entity).despawn();
            } else {
                // reflect the organism when it collides
                let mut reflect_x = false;
                let mut reflect_y = false;

                // only reflect if the organism's velocity is going in the opposite direction of the
                // collision
                match collision {
                    Collision::Left => reflect_x = organism_velocity.x > 0.0,
                    Collision::Right => reflect_x = organism_velocity.x < 0.0,
                    Collision::Top => reflect_y = organism_velocity.y < 0.0,
                    Collision::Bottom => reflect_y = organism_velocity.y > 0.0,
                    Collision::Inside => { /* do nothing */ }
                }

                // reflect velocity on the x-axis if we hit something on the x-axis
                if reflect_x {
                    organism_velocity.x = -organism_velocity.x;
                }

                // reflect velocity on the y-axis if we hit something on the y-axis
                if reflect_y {
                    organism_velocity.y = -organism_velocity.y;
                }
            }
        }
    }
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FoodTimer(Timer::from_seconds(0.2, TimerMode::Repeating)))
            .add_startup_system(startup)
            .add_event::<CollisionEvent>()
            .add_system(generate_food)
            .add_systems(
                (
                    check_for_collisions,
                    apply_velocity.before(check_for_collisions),
                    grow_organism.after(check_for_collisions),
                )
                    .in_schedule(CoreSchedule::FixedUpdate),
            )
            .insert_resource(FixedTime::new_from_secs(TIME_STEP));
    }
}
