use specs::prelude::*;

use crate::components::*;
use crate::resources::*;

use cgmath::Vector2;

use crate::graphics::Image as GraphicsImage;

const WIDTH: f32 = 480.0;
const HEIGHT: f32 = 640.0;
const PLAYER_SPEED: f32 = 250.0 / 60.0;
const PLAYER_BULLET_SPEED: f32 = 1000.0 / 60.0;

mod rendering;
mod bullets;

pub use rendering::*;
pub use bullets::*;

pub struct MoveEntities;

impl<'a> System<'a> for MoveEntities {
    type SystemData = (WriteStorage<'a, Position>, WriteStorage<'a, Movement>, ReadStorage<'a, FrozenUntil>, Read<'a, GameTime>);

    fn run(&mut self, (mut pos, mut mov, frozen, game_time): Self::SystemData) {
        for (mut pos, mov, _) in (&mut pos, &mut mov, !&frozen).join() {
            match mov {
                Movement::Linear(vector) => pos.0 += *vector,
                Movement::Falling { speed, down } => {
                    if *down {
                        pos.0.y += *speed;
                    } else {
                        pos.0.y -= *speed;
                    }

                    *speed += 0.0625;
                },
                Movement::FollowCurve(curve) => {
                    pos.0 = curve.step(pos.0);
                },
                Movement::FiringMove { speed, return_time, stop_time } => {
                    if *return_time <= game_time.total_time {
                        pos.0.y -= *speed;
                    } else if *stop_time > game_time.total_time {
                        pos.0.y += *speed;
                    }
                }
            }
        }
    }
}

fn min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

pub struct TogglePaused;

impl<'a> System<'a> for TogglePaused {
    type SystemData = (Write<'a, ControlsState>, Write<'a, Mode>);

    fn run(&mut self, (mut ctrl_state, mut mode): Self::SystemData) {
        if ctrl_state.pause.pressed {
            *mode = match *mode {
                Mode::Playing => Mode::Paused { selected: 0 },
                Mode::Paused { .. } => Mode::Playing,
                _ => *mode
            };
            ctrl_state.pause.pressed = false;
        }
    }
}

pub struct ControlMenu;

impl<'a> System<'a> for ControlMenu {
    type SystemData = (Write<'a, ControlsState>, Write<'a, Mode>);

    fn run(&mut self, (mut ctrl_state, mut mode): Self::SystemData) {
        if let Some(mut menu) = mode.as_menu(&ctrl_state) {
            let player_ctrl_state = ctrl_state.get_mut(Player::Single);

            if player_ctrl_state.down.pressed {
                menu.rotate_down();
                player_ctrl_state.down.pressed = false;
            }

            if player_ctrl_state.up.pressed {
                menu.rotate_up();
                player_ctrl_state.up.pressed = false;
            }

            let last_item = menu.items.len() - 1;

            if player_ctrl_state.fire.pressed {
                match *mode {
                    Mode::Paused { selected } => {
                        *mode = match selected {
                            0 => Mode::Playing,
                            1 => Mode::MainMenu { selected: 0 },
                            _ => unreachable!()
                        }
                    },
                    Mode::MainMenu { selected } => {
                        *mode = match selected {
                            0 => Mode::Stages { selected: 0, multiplayer: false },
                            1 => Mode::Controls { selected: 0 },
                            2 => Mode::Quit,
                            _ => unreachable!()
                        };
                    },
                    Mode::Stages { selected, multiplayer } => {
                        *mode = match selected {
                            0 => Mode::StageOne { multiplayer },
                            1 => Mode::StageTwo { multiplayer },
                            2 => Mode::Stages { selected, multiplayer: !multiplayer },
                            3 => Mode::MainMenu { selected: 0 },
                            _ => unreachable!()
                        }
                    },
                    Mode::Controls { selected } => {
                        if selected == last_item {
                            *mode = Mode::MainMenu { selected: 1 };
                        }
                    }
                    _ => {}
                }

                player_ctrl_state.fire.pressed = false;
            }
        }
    }
}

pub struct Control;

impl<'a> System<'a> for Control {
    type SystemData = (
        Entities<'a>, Read<'a, ControlsState>, Read<'a, GameTime>, Read<'a, LazyUpdate>, Write<'a, BulletSpawner>,
        ReadStorage<'a, Player>, WriteStorage<'a, Position>, WriteStorage<'a, Cooldown>, WriteStorage<'a, PowerBar>,
    );

    fn run(&mut self, (entities, ctrl_state, time, updater, mut bullet_spawner, player, mut position, mut cooldown, mut bar): Self::SystemData) {
        for (player, mut pos, cooldown, bar) in (&player, &mut position, &mut cooldown, &mut bar).join() {
            let player_ctrl_state = ctrl_state.get(*player);

            if player_ctrl_state.left.pressed {
                pos.0.x = max(pos.0.x - PLAYER_SPEED, 0.0);
            }

            if player_ctrl_state.right.pressed {
                pos.0.x = min(pos.0.x + PLAYER_SPEED, WIDTH);
            }

            if player_ctrl_state.up.pressed {
                pos.0.y = max(pos.0.y - PLAYER_SPEED, 0.0);
            }

            if player_ctrl_state.down.pressed {
                pos.0.y = min(pos.0.y + PLAYER_SPEED, HEIGHT);
            }

            if player_ctrl_state.fire.pressed && cooldown.is_ready(time.total_time) {
                for direction in &[-0.2_f32, -0.1, 0.0, 0.1, 0.2] {
                    bullet_spawner.0.push(BulletToBeSpawned {
                        pos: pos.0,
                        image: Image::from(GraphicsImage::PlayerBullet),
                        velocity: Vector2::new(direction.sin(), -direction.cos()) * PLAYER_BULLET_SPEED,
                        enemy: false,
                    });
                }
            }

            if player_ctrl_state.bomb.pressed && bar.empty() {
                updater.create_entity(&entities)
                    .with(Position(pos.0))
                    .with(Circle { radius: 0.0 })
                    .build();
            }
        }
    }
}

pub struct TickTime;

impl<'a> System<'a> for TickTime {
    type SystemData = (Entities<'a>, Write<'a, GameTime>, WriteStorage<'a, FrozenUntil>);

    fn run(&mut self, (entities, mut game_time, mut frozen): Self::SystemData) {
        game_time.total_time += 1.0 / 60.0;

        for (_, entry) in (&entities, frozen.entries()).join() {
            if let specs::storage::StorageEntry::Occupied(entry) = entry {
                if entry.get().0 <= game_time.total_time {
                    entry.remove();
                }
            }
        }
    } 
}

pub struct StartTowardsPlayer;

impl<'a> System<'a> for StartTowardsPlayer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, FrozenUntil>, ReadStorage<'a, Position>,
        WriteStorage<'a, TargetPlayer>, WriteStorage<'a, Movement>,
        Read<'a, PlayerPositions>,
    );

    fn run(&mut self, (entities, frozen, pos, mut target, mut movement, player_positions): Self::SystemData) {
        let mut rng = rand::thread_rng();
        
        for (entity, target, pos, _) in (&entities, target.entries(), &pos, !&frozen).join() {
            if let specs::storage::StorageEntry::Occupied(target) = target {
                let speed = target.get().0;
                target.remove();

                let player = player_positions.random(&mut rng);
                let rotation = (player.y - pos.0.y).atan2(player.x - pos.0.x);

                movement.insert(entity, Movement::Linear(Vector2::new(rotation.cos() * speed, rotation.sin() * speed)))
                    .unwrap();
            }
        }
    }
}

pub struct SetPlayerPositions;

impl<'a> System<'a> for SetPlayerPositions {
    type SystemData = (ReadStorage<'a, Position>, ReadStorage<'a, Player>, Write<'a, PlayerPositions>);

    fn run(&mut self, (pos, player, mut positions): Self::SystemData) {
        positions.0.clear();

        for (pos, _) in (&pos, &player).join() {
            positions.0.push(pos.0);
        }
    }
}

pub struct KillOffscreen;

impl<'a> System<'a> for KillOffscreen {
    type SystemData = (Entities<'a>, ReadStorage<'a, Position>, ReadStorage<'a, BeenOnscreen>, ReadStorage<'a, Image>);

    fn run(&mut self, (entities, pos, been_onscreen, image): Self::SystemData) {
        for (entity, pos, _, image) in (&entities, &pos, &been_onscreen, &image).join() {
            if !(is_onscreen(pos, *image)) {
                entities.delete(entity).unwrap();
            }
        }
    }
}

pub struct AddOnscreen;

impl<'a> System<'a> for AddOnscreen {
    type SystemData = (Entities<'a>, ReadStorage<'a, Position>, ReadStorage<'a, Image>, ReadStorage<'a, DieOffscreen>, WriteStorage<'a, BeenOnscreen>);

    fn run(&mut self, (entities, pos, image, die_offscreen, mut been_onscreen): Self::SystemData) {
        for (entity, pos, image, _) in (&entities, &pos, &image, &die_offscreen).join() {
            if is_onscreen(pos, *image) {
                been_onscreen.insert(entity, BeenOnscreen).unwrap();
            }
        }
    }
}

fn is_onscreen(pos: &Position, image: Image) -> bool {
    let size = image.size() / 2.0;
    !(pos.0.y + size.y <= 0.0 || pos.0.y - size.y >= HEIGHT || pos.0.x + size.x <= 0.0 || pos.0.x - size.x >= WIDTH)
}


pub struct CollectOrbs;

impl<'a> System<'a> for CollectOrbs {
    type SystemData = (Entities<'a>, ReadStorage<'a, PowerOrb>, ReadStorage<'a, Position>, ReadStorage<'a, Hitbox>, WriteStorage<'a, PowerBar>);

    fn run(&mut self, (entities, orb, position, hitbox, mut power_bar): Self::SystemData) {
        for (player_pos, player_hit, power_bar) in (&position, &hitbox, &mut power_bar).join() {
            for (orb_entity, orb, orb_pos, orb_hit) in (&entities, &orb, &position, &hitbox).join() {
                if is_touching(player_pos.0, player_hit.0, orb_pos.0, orb_hit.0).is_some() {
                    entities.delete(orb_entity).unwrap();
                    power_bar.add(orb.0);
                }
            }
        }
    }
}

fn is_touching(pos_a: Vector2<f32>, hit_a: Vector2<f32>, pos_b: Vector2<f32>, hit_b: Vector2<f32>) -> Option<Vector2<f32>> {
    if hit_a == Vector2::new(0.0, 0.0) && hit_b == Vector2::new(0.0, 0.0) {
        return None;
    }

    let a_t_l = pos_a - hit_a / 2.0;
    let a_b_r = pos_a + hit_a / 2.0;
    
    let b_t_l = pos_b - hit_b / 2.0;
    let b_b_r = pos_b + hit_b / 2.0;
    
    let is_touching = !(
        a_t_l.x > b_b_r.x  || a_b_r.x  < b_t_l.x ||
        a_t_l.y  > b_b_r.y || a_b_r.y < b_t_l.y
    );

    if is_touching {
        Some(if hit_a.x * hit_a.y > hit_b.x * hit_b.y {
            pos_b
        } else {
            pos_a
        })
    } else {
        None
    }
}
