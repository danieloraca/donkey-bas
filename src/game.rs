use std::time::SystemTime;

pub(crate) const WIDTH: usize = 320;
pub(crate) const HEIGHT: usize = 200;
pub(crate) const FPS: u64 = 60;

pub(crate) const ROAD_TOP: i32 = 54;
pub(crate) const ROAD_BOTTOM: i32 = 199;
const ROAD_TOP_W: i32 = 72;
const ROAD_BOTTOM_W: i32 = 278;
pub(crate) const ROAD_CENTER_X: i32 = (WIDTH as i32) / 2;

pub(crate) const SPRITE_SCALE: i32 = 2;
pub(crate) const CAR_W: i32 = 16 * SPRITE_SCALE;
pub(crate) const CAR_H: i32 = 12 * SPRITE_SCALE;
pub(crate) const DONKEY_BASE_W: i32 = 16;
pub(crate) const DONKEY_BASE_H: i32 = 12;
pub(crate) const CAR_Y: i32 = 154;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    Left,
    Right,
}

impl Lane {
    pub(crate) fn direction(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum GameEvent {
    Score,
    Crash,
}

pub(crate) struct Game {
    pub(crate) car_lane: Lane,
    pub(crate) car_position: f32,
    pub(crate) donkey_lane: Lane,
    pub(crate) donkey_y: f32,
    pub(crate) road_scroll: f32,
    pub(crate) city_tick: u64,
    pub(crate) crash_started_at: u64,
    pub(crate) score: u32,
    pub(crate) over: bool,
    pub(crate) started: bool,
    pub(crate) sound_on: bool,
    rng: u32,
}

impl Game {
    pub(crate) fn new(seed: u32) -> Self {
        Self {
            car_lane: Lane::Left,
            car_position: -1.0,
            donkey_lane: Lane::Right,
            donkey_y: ROAD_TOP as f32,
            road_scroll: 0.0,
            city_tick: 0,
            crash_started_at: 0,
            score: 0,
            over: false,
            started: false,
            sound_on: true,
            rng: seed.max(1),
        }
    }

    pub(crate) fn reset(&mut self, seed: u32) {
        let sound_on = self.sound_on;
        let city_tick = self.city_tick;
        *self = Self::new(seed);
        self.sound_on = sound_on;
        self.city_tick = city_tick;
        self.started = true;
        self.spawn_donkey();
    }

    fn next_u32(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    pub(crate) fn spawn_donkey(&mut self) {
        self.donkey_lane = if self.next_u32() >> 31 == 0 {
            Lane::Left
        } else {
            Lane::Right
        };
        self.donkey_y = ROAD_TOP as f32;
    }

    fn donkey_speed(&self) -> f32 {
        55.0 + 30.0 * (1.0 + self.score as f32 / 5.0).ln()
    }

    pub(crate) fn level(&self) -> u32 {
        1 + self.score / 5
    }

    pub(crate) fn update_ambient_motion(&mut self) {
        let target = self.car_lane.direction() as f32;
        let distance = target - self.car_position;
        self.car_position += distance * 0.20;
        if distance.abs() < 0.002 {
            self.car_position = target;
        }
    }

    pub(crate) fn update(&mut self) -> Option<GameEvent> {
        if !self.started || self.over {
            return None;
        }

        let step = self.donkey_speed() / FPS as f32;
        self.donkey_y += step;
        self.road_scroll = (self.road_scroll + step) % 364.0;

        if self.collides() {
            self.over = true;
            self.crash_started_at = self.city_tick;
            return Some(GameEvent::Crash);
        }

        if self.donkey_y > ROAD_BOTTOM as f32 {
            self.score += 1;
            self.spawn_donkey();
            return Some(GameEvent::Score);
        }

        None
    }

    fn collides(&self) -> bool {
        let scale = donkey_scale(self.donkey_y);
        let donkey_top = self.donkey_y;
        let donkey_bottom = self.donkey_y + DONKEY_BASE_H as f32 * scale - 1.0;
        let car_top = CAR_Y as f32;
        let car_bottom = (CAR_Y + CAR_H - 1) as f32;
        if donkey_top > car_bottom || donkey_bottom < car_top {
            return false;
        }

        let donkey_width = scaled(DONKEY_BASE_W, scale);
        let donkey_x = lane_x(
            self.donkey_lane,
            self.donkey_y.round() as i32,
            donkey_width,
            self.road_scroll,
        );
        let car_x = road_position_x(self.car_position, CAR_Y, CAR_W, self.road_scroll);
        donkey_x + donkey_width - 4 >= car_x + 4 && donkey_x + 4 <= car_x + CAR_W - 4
    }
}

pub(crate) fn seed_now() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0xC0FFEE_u32)
}

pub(crate) fn road_half_width(y: i32) -> i32 {
    let depth = ((y - ROAD_TOP) as f32 / (ROAD_BOTTOM - ROAD_TOP) as f32).clamp(0.0, 1.0);
    let top_half = ROAD_TOP_W as f32 / 2.0;
    let bottom_half = ROAD_BOTTOM_W as f32 / 2.0;
    (top_half + (bottom_half - top_half) * depth.powf(0.78)) as i32
}

pub(crate) fn road_center(_y: i32, _scroll: f32) -> i32 {
    ROAD_CENTER_X
}

pub(crate) fn road_position_x(position: f32, y: i32, width: i32, scroll: f32) -> i32 {
    let half = road_half_width(y);
    road_center(y, scroll) + (position * half as f32 * 0.5) as i32 - width / 2
}

pub(crate) fn lane_x(lane: Lane, y: i32, width: i32, scroll: f32) -> i32 {
    road_position_x(lane.direction() as f32, y, width, scroll)
}

pub(crate) fn donkey_scale(y: f32) -> f32 {
    let progress = ((y - ROAD_TOP as f32) / (ROAD_BOTTOM - ROAD_TOP) as f32).clamp(0.0, 1.0);
    1.25 + progress * 1.25
}

pub(crate) fn scaled(value: i32, scale: f32) -> i32 {
    (value as f32 * scale).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawned_lanes_do_not_follow_a_forced_alternating_pattern() {
        let mut game = Game::new(0x1234_5678);
        let lanes: Vec<Lane> = (0..32)
            .map(|_| {
                game.spawn_donkey();
                game.donkey_lane
            })
            .collect();

        assert!(lanes.windows(2).any(|pair| pair[0] == pair[1]));
        assert!(lanes.contains(&Lane::Left));
        assert!(lanes.contains(&Lane::Right));
    }

    #[test]
    fn difficulty_increases_forever_with_diminishing_gains() {
        let mut game = Game::new(1);
        assert_eq!(game.level(), 1);
        assert_eq!(game.donkey_speed(), 55.0);

        game.score = 5;
        let level_two_speed = game.donkey_speed();
        game.score = 20;
        let level_five_speed = game.donkey_speed();
        game.score = 100;
        let late_game_speed = game.donkey_speed();
        game.score = 1_000;
        let very_late_game_speed = game.donkey_speed();

        assert!(level_two_speed > 55.0);
        assert!(level_five_speed > level_two_speed);
        assert!(late_game_speed > level_five_speed);
        assert!(very_late_game_speed > late_game_speed);
        assert!(very_late_game_speed - late_game_speed < late_game_speed - 55.0);
    }

    #[test]
    fn road_stays_centered_at_every_depth() {
        assert_eq!(road_center(ROAD_BOTTOM, 0.0), ROAD_CENTER_X);
        assert_eq!(road_center(ROAD_BOTTOM, 173.0), ROAD_CENTER_X);
        assert_eq!(road_center(ROAD_TOP, 0.0), ROAD_CENTER_X);
        assert_eq!(road_center(ROAD_TOP, 100.0), ROAD_CENTER_X);
    }

    #[test]
    fn steering_eases_toward_the_selected_lane() {
        let mut game = Game::new(1);
        game.car_lane = Lane::Right;
        game.update_ambient_motion();
        assert!(game.car_position > -1.0 && game.car_position < 1.0);

        for _ in 0..100 {
            game.update_ambient_motion();
        }
        assert_eq!(game.car_position, 1.0);
    }
}
