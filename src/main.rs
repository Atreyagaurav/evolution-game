use std::io::Write;

use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    sprite::MaterialMesh2dBundle,
};

use rand;

const TIME_STEP: f32 = 1.0 / 60.0;
const SIMULATION_SPEED: f32 = 5.0;
const TOP_BOUNDARY: f32 = 300.0;
const BOTTOM_BOUNDARY: f32 = -300.0;
const RIGHT_BOUNDARY: f32 = 600.0;
const LEFT_BOUNDARY: f32 = -600.0;
const BOUNDARY_THICKNESS: f32 = 4.0;

const BACKGROUND_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const BOUNDARY_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const FOOD_COLOR: Color = Color::rgb(0.1, 0.4, 0.1);

const ORGANISM_SIZE: Vec3 = Vec3::new(15.0, 15.0, 0.0);
const PHEROMONE_SIZE: Vec3 = Vec3::new(4.0, 4.0, 0.0);
const FOOD_SIZE: Vec3 = Vec3::new(4.0, 4.0, 0.0);

const ORGANISM_DEFAULT_SPEED: f32 = 8.0;
const ORGANISM_VISION: f32 = 100.0;
const INITIAL_POPULATION: usize = 50;
const FOOD_PER_TIMESTEP: usize = 2;
const PREGNANT_PROBABILITY: f32 = 0.5;
const CHILDREN_PER_PREGNANCY: usize = 10;

const PREGNANCY_ENERGY_MINIMUM: f32 = 2.0;
const ORGANISM_MIN_ENERGY: f32 = 0.2;
const ORGANISM_MAX_ENERGY: f32 = 4.0;
const ORGANISM_DEFAULT_LIFETIME: usize = 100;
const PHEROMONE_DEFAULT_LIFETIME: usize = ORGANISM_DEFAULT_LIFETIME / 10;
const FERTILE_AGE: usize = ORGANISM_DEFAULT_LIFETIME / 4;
const FOOD_LIFETIME: usize = 100;
const MUTATION_RATE: f32 = 0.2;

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
struct Pheromone;

#[derive(Component)]
struct Energy(f32);

#[derive(Component, Debug)]
struct GeneInfo([f32; 27]);

impl Default for GeneInfo {
    fn default() -> Self {
        let mut gene: [f32; 27] = rand::random();
        gene = gene.map(|g| (g - 0.5) * 2.0);
        gene[0] /= 2.0;
        gene[1] /= 2.0;
        gene[2] /= 2.0;
        Self(gene)
    }
}

impl GeneInfo {
    fn planned() -> Self {
        let mut gene: [f32; 27] = [0.0; 27];
        // slow down if food is on left or right
        gene[16] = -0.1;
        gene[18] = -0.1;
        // speed up if there is food on the front
        gene[17] = 1.0;
        // go left if food is on left
        gene[8] = 0.5;
        // go right if food is on right
        gene[9] = -0.5;
        Self(gene)
    }

    fn mutate(&self) -> Self {
        let new_gene = self.0.map(|g| {
            if rand::random::<f32>() < MUTATION_RATE {
                (g + rand::random::<f32>() / 2.0 - 0.25).clamp(-1.0, 1.0)
            } else {
                g
            }
        });
        Self(new_gene)
    }

    fn process(&self, inputs: &[f32; 8]) -> [f32; 3] {
        let gene = Vec::from(self.0);
        let delta_x: f32 = (gene[0]
            + gene[3..=10]
                .iter()
                .zip(inputs)
                .map(|(c, i)| c * i)
                .sum::<f32>())
        .clamp(-1.0, 1.0);
        let delta_y: f32 = (gene[1]
            + gene[11..=18]
                .iter()
                .zip(inputs)
                .map(|(c, i)| c * i)
                .sum::<f32>())
        .clamp(-1.0, 1.0);
        let delta_a: f32 = (gene[2]
            + gene[19..=26]
                .iter()
                .zip(inputs)
                .map(|(c, i)| c * i)
                .sum::<f32>())
        .clamp(-1.0, 1.0);
        [delta_x, delta_y, delta_a]
    }

    fn color(&self) -> Color {
        Color::rgb(
            (self.0[0] + 1.0) / 2.0,
            (self.0[1] + 1.0) / 2.0,
            (self.0[2] + 1.0) / 2.0,
        )
    }
}

#[derive(Component)]
struct Age(usize);

#[derive(Component)]
struct Lifetime(usize);

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Pregnant(bool);

#[derive(Component, Deref, DerefMut)]
struct Direction(Vec2);

#[derive(Component)]
struct Speed(f32);

#[derive(Component)]
struct Collider;

enum CollisionEvent {
    Wall,
    Food,
}

#[derive(Resource)]
struct FoodTimer(Timer);

#[derive(Resource)]
struct LogTimer(Timer);

#[derive(Resource)]
struct SensoryTimer(Timer);

#[derive(Resource)]
struct AgeTimer(Timer);

#[derive(Resource)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Resource)]
struct FeedingSound(Handle<AudioSource>);

fn random_position() -> Vec3 {
    let (x, y): (f32, f32) = (rand::random(), rand::random());
    Vec3::new(
        (x - 0.5) * 2.0 * RIGHT_BOUNDARY,
        (y - 0.5) * 2.0 * TOP_BOUNDARY,
        0.0,
    )
}

fn random_direction() -> Vec2 {
    let (x, y): (f32, f32) = (rand::random(), rand::random());
    let v = Vec2::new(x - 0.5, y - 0.5);
    v / v.length()
}

fn rotate_direction(direction: &mut Vec2, angle: f32) {
    let x = angle.cos();
    let y = angle.sin();
    direction.x = x * direction.x + y * direction.y;
    direction.y = -y * direction.x + x * direction.y;
    if direction.length() > 0.01 {
        direction.x /= direction.length();
        direction.y /= direction.length();
    } else {
        direction.x = 1.0 / (2.0_f32).sqrt();
        direction.y = 1.0 / (2.0_f32).sqrt();
    }
}

fn _align_direction(direction: &mut Vec2, delta: &Vec2) {
    let angle = direction.angle_between(*delta);
    if angle < 0.5 || angle > 5.7 {
        let r = delta.length();
        direction.x = delta.x / r;
        direction.y = delta.y / r;
    } else {
        rotate_direction(direction, angle.clamp(-0.5, 0.5))
    }
}

fn adjust_direction(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<SensoryTimer>,
    mut organism_query: Query<
        (
            &Transform,
            &mut Direction,
            &mut Speed,
            &Energy,
            &Lifetime,
            &GeneInfo,
        ),
        With<Organism>,
    >,
    food_query: Query<&Transform, With<Food>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for (transform, mut direction, mut speed, energy, lifetime, gene) in &mut organism_query {
            let mut foods: [f32; 3] = [0.0, 0.0, 0.0];
            for food_transform in &food_query {
                let food_pos = food_transform.translation;
                let dir = (food_pos - transform.translation).truncate();
                let dist = dir.length();
                if dist < ORGANISM_VISION {
                    let alpha = dir.angle_between(**direction);
                    let food_val = (ORGANISM_VISION * 0.5) / (ORGANISM_VISION + dist);
                    if alpha > -0.1 && alpha < 0.1 {
                        foods[1] += food_val;
                    } else if alpha < 1.0 && alpha > 0.1 {
                        foods[0] += food_val;
                    } else if alpha > -1.0 && alpha < -0.1 {
                        foods[2] += food_val;
                    }
                }
            }

            let x_pos = transform.translation.x;
            let y_pos = transform.translation.y;
            let x_pos = (x_pos - LEFT_BOUNDARY) / (RIGHT_BOUNDARY - LEFT_BOUNDARY);
            let y_pos = (y_pos - BOTTOM_BOUNDARY) / (TOP_BOUNDARY - BOTTOM_BOUNDARY);
            if (x_pos < 0.1 && direction.x < 0.0)
                || (x_pos > 0.9 && direction.x > 0.0)
                || (y_pos < 0.1 && direction.y < 0.0)
                || (y_pos > 0.9 && direction.y > 0.0)
            {
                foods[1] = -1.0;
            }
            let inputs: [f32; 8] = [
                speed.0 / ORGANISM_DEFAULT_SPEED,
                x_pos,
                y_pos,
                (energy.0 - ORGANISM_MIN_ENERGY) / (ORGANISM_MAX_ENERGY - ORGANISM_MIN_ENERGY),
                lifetime.0 as f32 / ORGANISM_DEFAULT_LIFETIME as f32,
                foods[0].clamp(0.0, 1.0),
                foods[1].clamp(0.0, 1.0),
                foods[2].clamp(0.0, 1.0),
            ];
            let output = gene.process(&inputs);
            rotate_direction(&mut direction, output[0]);
            speed.0 = (speed.0 + output[1]).clamp(0.0, ORGANISM_DEFAULT_SPEED);

            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::default().into()).into(),
                    material: materials.add(ColorMaterial::from(gene.color())),
                    transform: Transform::from_translation(transform.translation)
                        .with_scale(PHEROMONE_SIZE),
                    ..default()
                },
                Pheromone,
                Lifetime(PHEROMONE_DEFAULT_LIFETIME),
                Age(1),
            ));
        }
    }
}

fn pheromone_fade(
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(&Handle<ColorMaterial>, &Age, &Lifetime), With<Pheromone>>,
) {
    for (handle, age, lifetime) in &query {
        let mut col = materials.get_mut(handle).unwrap().color;
        col.set_a(1.0 - age.0 as f32 / lifetime.0 as f32);
    }
}

fn apply_direction(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Direction, &Speed, &mut Energy)>,
) {
    for (entity, mut transform, direction, speed, mut energy) in &mut query {
        if transform.translation.x < LEFT_BOUNDARY
            || transform.translation.x > RIGHT_BOUNDARY
            || transform.translation.y < BOTTOM_BOUNDARY
            || transform.translation.y > TOP_BOUNDARY
        {
            commands.entity(entity).despawn();
        }
        let deltax = direction.x * speed.0 * TIME_STEP * SIMULATION_SPEED;
        let deltay = direction.y * speed.0 * TIME_STEP * SIMULATION_SPEED;

        transform.translation.x += deltax;
        transform.translation.y += deltay;

        // propotional energy consumption based on size
        energy.0 *= 0.999;
        // energy comsumption based on speed
        energy.0 -= speed.0.powi(2) / 50000000.0;
    }
}

fn age_progression(
    time: Res<Time>,
    mut timer: ResMut<AgeTimer>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Age, &Lifetime)>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for (entity, mut age, lifetime) in &mut query {
            if age.0 > lifetime.0 as usize {
                commands.entity(entity).despawn();
            } else {
                age.0 += 1;
            }
        }
    }
}

fn log_things(
    time: Res<Time>,
    mut timer: ResMut<LogTimer>,
    query: Query<(&GeneInfo, &Direction, &Speed), With<Organism>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let file = std::fs::File::create("organisms.txt").unwrap();
        let mut file = std::io::BufWriter::new(file);
        for (gene, direction, speed) in &query {
            file.write(
                format!(
                    "{},{} ({}) <- {:?}\n",
                    direction.x, direction.y, speed.0, gene.0,
                )
                .as_bytes(),
            )
            .unwrap();
        }
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Sound
    let collision_sound = asset_server.load("sounds/collision.ogg");
    commands.insert_resource(CollisionSound(collision_sound));
    let feeding_sound = asset_server.load("sounds/feeding.ogg");
    commands.insert_resource(FeedingSound(feeding_sound));

    commands.spawn(Camera2dBundle::default());
    // Boundarys
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Left));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Right));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Bottom));
    commands.spawn(BoundaryBundle::new(BoundaryLocation::Top));

    // Organism
    for _ in 0..INITIAL_POPULATION {
        let gene = GeneInfo::planned();
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::default().into()).into(),
                material: materials.add(ColorMaterial::from(gene.color())),
                transform: Transform::from_translation(random_position()).with_scale(ORGANISM_SIZE),
                ..default()
            },
            Organism,
            gene,
            Lifetime(ORGANISM_DEFAULT_LIFETIME),
            Speed(ORGANISM_DEFAULT_SPEED),
            Energy(1.0),
            Age(1),
            Pregnant(false),
            Direction(random_direction()),
        ));
    }
}

fn generate_food(
    time: Res<Time>,
    mut timer: ResMut<FoodTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for _ in 0..FOOD_PER_TIMESTEP {
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::default().into()).into(),
                    material: materials.add(ColorMaterial::from(FOOD_COLOR)),
                    transform: Transform::from_translation(random_position()).with_scale(FOOD_SIZE),
                    ..default()
                },
                Food,
                Age(1),
                Lifetime(FOOD_LIFETIME),
                Energy(0.1),
                Collider,
            ));
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

fn grow_organism(
    mut commands: Commands,
    mut organism_query: Query<
        (
            Entity,
            &mut Transform,
            &GeneInfo,
            &mut Energy,
            &mut Pregnant,
        ),
        With<Organism>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (organism, mut organism_transform, gene_info, mut organism_energy, mut organism_pregnant) in
        &mut organism_query
    {
        if organism_energy.0 < ORGANISM_MIN_ENERGY || organism_energy.0 > ORGANISM_MAX_ENERGY {
            commands.entity(organism).despawn();
        } else if organism_pregnant.0 {
            organism_energy.0 = 1.0;
            organism_pregnant.0 = false;
            for _ in 0..CHILDREN_PER_PREGNANCY {
                let gene = gene_info.mutate();
                commands.spawn((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::default().into()).into(),
                        material: materials.add(ColorMaterial::from(gene.color())),
                        transform: Transform::from_translation(organism_transform.translation)
                            .with_scale(ORGANISM_SIZE),
                        ..default()
                    },
                    Organism,
                    Energy(0.5),
                    Age(1),
                    gene,
                    Lifetime(ORGANISM_DEFAULT_LIFETIME),
                    Speed(ORGANISM_DEFAULT_SPEED),
                    Pregnant(false),
                    Direction(random_direction()),
                ));
            }
        }
        organism_transform.scale = ORGANISM_SIZE * organism_energy.0.sqrt();
    }
}

fn check_for_collisions(
    mut commands: Commands,
    mut organism_query: Query<
        (&mut Direction, &Transform, &Age, &mut Energy, &mut Pregnant),
        With<Organism>,
    >,
    collider_query: Query<(Entity, &Transform, Option<&Food>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (
        mut organism_direction,
        organism_transform,
        organism_age,
        mut organism_energy,
        mut organism_pregnant,
    ) in &mut organism_query
    {
        let organism_size = organism_transform.scale.truncate();

        for (collider_entity, transform, maybe_food) in &collider_query {
            let collision = collide(
                organism_transform.translation,
                organism_size,
                transform.translation,
                transform.scale.truncate(),
            );
            if let Some(collision) = collision {
                if maybe_food.is_some() {
                    commands.entity(collider_entity).despawn();
                    collision_events.send(CollisionEvent::Food);
                    organism_energy.0 += 0.2;
                    if organism_energy.0 > PREGNANCY_ENERGY_MINIMUM
                        && organism_age.0 > FERTILE_AGE
                        && rand::random::<f32>() < PREGNANT_PROBABILITY
                    {
                        organism_pregnant.0 = true;
                    }
                } else {
                    // reflect the organism when it collides
                    collision_events.send(CollisionEvent::Wall);
                    let mut reflect_x = false;
                    let mut reflect_y = false;

                    // only reflect if the organism's direction is going in the opposite direction of the
                    // collision
                    match collision {
                        Collision::Left => reflect_x = organism_direction.x > 0.0,
                        Collision::Right => reflect_x = organism_direction.x < 0.0,
                        Collision::Top => reflect_y = organism_direction.y < 0.0,
                        Collision::Bottom => reflect_y = organism_direction.y > 0.0,
                        Collision::Inside => { /* do nothing */ }
                    }

                    // reflect direction on the x-axis if we hit something on the x-axis
                    if reflect_x {
                        organism_direction.x = -organism_direction.x;
                    }

                    // reflect direction on the y-axis if we hit something on the y-axis
                    if reflect_y {
                        organism_direction.y = -organism_direction.y;
                    }
                }
            }
        }
    }
}

fn _play_collision_sound(
    mut collision_events: EventReader<CollisionEvent>,
    audio: Res<Audio>,
    collision: Res<CollisionSound>,
    feeding: Res<FeedingSound>,
) {
    if !collision_events.is_empty() {
        for event in &mut collision_events {
            match event {
                CollisionEvent::Food => {
                    audio.play(feeding.0.clone());
                }
                CollisionEvent::Wall => (),
                _ => {
                    audio.play(collision.0.clone());
                }
            };
        }
        collision_events.clear();
    }
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FoodTimer(Timer::from_seconds(
            0.2 / SIMULATION_SPEED,
            TimerMode::Repeating,
        )))
        .insert_resource(SensoryTimer(Timer::from_seconds(
            0.5 / SIMULATION_SPEED,
            TimerMode::Repeating,
        )))
        .insert_resource(AgeTimer(Timer::from_seconds(
            1.0 / SIMULATION_SPEED,
            TimerMode::Repeating,
        )))
        .insert_resource(LogTimer(Timer::from_seconds(
            10.0 / SIMULATION_SPEED,
            TimerMode::Repeating,
        )))
        .add_startup_system(startup)
        .add_event::<CollisionEvent>()
        .add_systems(
            (
                pheromone_fade,
                log_things,
                generate_food,
                age_progression,
                check_for_collisions,
                apply_direction.before(adjust_direction),
                grow_organism.after(check_for_collisions),
                // play_collision_sound.after(check_for_collisions),
                adjust_direction.after(check_for_collisions),
            )
                .in_schedule(CoreSchedule::FixedUpdate),
        )
        .insert_resource(FixedTime::new_from_secs(TIME_STEP));
    }
}
