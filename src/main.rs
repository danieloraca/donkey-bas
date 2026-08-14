use std::time::{Duration, Instant, SystemTime};

use font8x8::{BASIC_FONTS, UnicodeFonts};
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SCALE: u32 = 3;
const FPS: u64 = 60;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FPS);

const SKY: [u8; 3] = [0x08, 0x0D, 0x22];
const SKY_GLOW: [u8; 3] = [0x2C, 0x1E, 0x4E];
const GROUND: [u8; 3] = [0x0B, 0x34, 0x2D];
const ROAD: [u8; 3] = [0x20, 0x24, 0x32];
const ROAD_EDGE: [u8; 3] = [0xFF, 0x4F, 0x81];
const CYAN: [u8; 3] = [0x25, 0xE6, 0xD2];
const CYAN_DARK: [u8; 3] = [0x08, 0x61, 0x70];
const YELLOW: [u8; 3] = [0xFF, 0xD1, 0x66];
const DONKEY: [u8; 3] = [0xD8, 0x91, 0x56];
const SHADOW: [u8; 3] = [0x08, 0x0A, 0x12];
const BUILDING_BACK: [u8; 3] = [0x12, 0x18, 0x38];
const BUILDING_MID: [u8; 3] = [0x1A, 0x20, 0x4A];
const BUILDING_FRONT: [u8; 3] = [0x13, 0x19, 0x31];
const TREE_DARK: [u8; 3] = [0x08, 0x45, 0x3C];
const TREE_LIGHT: [u8; 3] = [0x10, 0x78, 0x58];
const TREE_TRUNK: [u8; 3] = [0x73, 0x43, 0x35];
const WHITE: [u8; 3] = [0xF4, 0xF7, 0xFF];

const ROAD_TOP: i32 = 54;
const ROAD_BOTTOM: i32 = 199;
const ROAD_TOP_W: i32 = 72;
const ROAD_BOTTOM_W: i32 = 278;
const ROAD_CENTER_X: i32 = (WIDTH as i32) / 2;

const SPRITE_SCALE: i32 = 2;
const CAR_W: i32 = 16 * SPRITE_SCALE;
const CAR_H: i32 = 12 * SPRITE_SCALE;
const DONKEY_BASE_W: i32 = 16;
const DONKEY_BASE_H: i32 = 12;
const CAR_Y: i32 = 154;

const CAR_SPRITE: [&str; 12] = [
    "0000001111000000",
    "0000011111100000",
    "0000111111110000",
    "0001111111111000",
    "0011111111111100",
    "0111111111111110",
    "1111001111001111",
    "1111111111111111",
    "1111111111111111",
    "0110000000000110",
    "0110000000000110",
    "0011000000001100",
];

const DONKEY_SPRITE: [&str; 12] = [
    "0000001100110000",
    "0000011111110000",
    "0000111111111000",
    "0001111111111100",
    "0011111111111110",
    "0111111111111111",
    "0111111111111111",
    "0011111111111110",
    "0001111111111100",
    "0001111001111100",
    "0000110000111000",
    "0000110000111000",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Left,
    Right,
}

impl Lane {
    fn direction(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

struct Game {
    car_lane: Lane,
    donkey_lane: Lane,
    donkey_y: f32,
    road_scroll: f32,
    city_tick: u64,
    score: u32,
    over: bool,
    started: bool,
    sound_on: bool,
    rng: u32,
}

impl Game {
    fn new(seed: u32) -> Self {
        Self {
            car_lane: Lane::Left,
            donkey_lane: Lane::Right,
            donkey_y: ROAD_TOP as f32,
            road_scroll: 0.0,
            city_tick: 0,
            score: 0,
            over: false,
            started: false,
            sound_on: true,
            rng: seed.max(1),
        }
    }

    fn reset(&mut self, seed: u32) {
        let sound_on = self.sound_on;
        let city_tick = self.city_tick;
        *self = Self::new(seed);
        self.sound_on = sound_on;
        self.city_tick = city_tick;
        self.started = true;
        self.spawn_donkey();
    }

    fn next_u32(&mut self) -> u32 {
        // Xorshift32 has usable bits throughout its output. The previous LCG's
        // lowest bit flipped on every call, forcing a left/right pattern.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn spawn_donkey(&mut self) {
        self.donkey_lane = if self.next_u32() >> 31 == 0 {
            Lane::Left
        } else {
            Lane::Right
        };
        self.donkey_y = ROAD_TOP as f32;
    }

    fn donkey_speed(&self) -> f32 {
        (55.0 + self.score as f32 * 4.0).min(135.0)
    }

    fn level(&self) -> u32 {
        1 + self.score / 5
    }
}

fn seed_now() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0xC0FFEE_u32)
}

#[derive(Clone, Copy)]
enum SoundEffect {
    Start,
    Move(Lane),
    Score,
    Crash,
    Toggle,
}

struct SoundEngine {
    queue: AudioQueue<f32>,
    sample_rate: f32,
}

impl SoundEngine {
    fn new(audio: &AudioSubsystem) -> Result<Self, String> {
        let desired = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1),
            samples: Some(512),
        };
        let queue = audio.open_queue::<f32, _>(None, &desired)?;
        let sample_rate = queue.spec().freq as f32;
        queue.resume();
        Ok(Self { queue, sample_rate })
    }

    fn tone(&self, output: &mut Vec<f32>, frequency: f32, milliseconds: u32, volume: f32) {
        let count = (self.sample_rate * milliseconds as f32 / 1_000.0) as usize;
        for i in 0..count {
            let phase = (i as f32 * frequency / self.sample_rate).fract();
            let square = if phase < 0.5 { 1.0 } else { -1.0 };
            let attack = (i as f32 / 48.0).min(1.0);
            let release = ((count - i) as f32 / 120.0).min(1.0);
            output.push(square * volume * attack * release);
        }
    }

    fn rest(&self, output: &mut Vec<f32>, milliseconds: u32) {
        let count = (self.sample_rate * milliseconds as f32 / 1_000.0) as usize;
        output.resize(output.len() + count, 0.0);
    }

    fn noise(&self, output: &mut Vec<f32>, milliseconds: u32, volume: f32) {
        let count = (self.sample_rate * milliseconds as f32 / 1_000.0) as usize;
        let mut lfsr = 0xACE1_u16;
        for i in 0..count {
            let bit = (lfsr ^ (lfsr >> 1)) & 1;
            lfsr = (lfsr >> 1) | (bit << 15);
            let sample = if lfsr & 1 == 0 { -1.0 } else { 1.0 };
            let envelope = 1.0 - i as f32 / count as f32;
            output.push(sample * volume * envelope);
        }
    }

    fn play(&self, enabled: bool, effect: SoundEffect) {
        if !enabled {
            return;
        }

        let mut samples = Vec::new();
        match effect {
            SoundEffect::Start => {
                self.queue.clear();
                for frequency in [220.0, 330.0, 440.0, 660.0] {
                    self.tone(&mut samples, frequency, 55, 0.16);
                    self.rest(&mut samples, 12);
                }
            }
            SoundEffect::Move(lane) => {
                let frequency = if lane == Lane::Left { 330.0 } else { 440.0 };
                self.tone(&mut samples, frequency, 32, 0.09);
            }
            SoundEffect::Score => {
                self.tone(&mut samples, 660.0, 55, 0.14);
                self.tone(&mut samples, 880.0, 90, 0.16);
            }
            SoundEffect::Crash => {
                self.queue.clear();
                self.tone(&mut samples, 140.0, 90, 0.18);
                self.noise(&mut samples, 280, 0.20);
            }
            SoundEffect::Toggle => self.tone(&mut samples, 520.0, 70, 0.12),
        }

        // Avoid building a long backlog if several events happen at once.
        if self.queue.size() < (self.sample_rate as u32 * 2) {
            let _ = self.queue.queue_audio(&samples);
        }
    }
}

fn clear(buffer: &mut [u8], color: [u8; 3]) {
    for px in buffer.chunks_exact_mut(3) {
        px.copy_from_slice(&color);
    }
}

fn fill_rect(buffer: &mut [u8], x: i32, y: i32, w: i32, h: i32, color: [u8; 3]) {
    let left = x.max(0) as usize;
    let right = (x + w).min(WIDTH as i32).max(0) as usize;
    if left >= right {
        return;
    }
    for py in y.max(0)..(y + h).min(HEIGHT as i32) {
        let start = (py as usize * WIDTH + left) * 3;
        let end = (py as usize * WIDTH + right) * 3;
        for pixel in buffer[start..end].chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
    }
}

fn draw_char(buffer: &mut [u8], x: i32, y: i32, c: char, color: [u8; 3], scale: i32) {
    if let Some(glyph) = BASIC_FONTS.get(c) {
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 == 1 {
                    fill_rect(
                        buffer,
                        x + (col * scale),
                        y + (row as i32 * scale),
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}

fn draw_text(buffer: &mut [u8], x: i32, y: i32, text: &str, color: [u8; 3], scale: i32) {
    let mut cursor = x;
    for c in text.chars() {
        draw_char(buffer, cursor, y, c, color, scale);
        cursor += 8 * scale;
    }
}

fn road_half_width(y: i32) -> i32 {
    let t = ((y - ROAD_TOP) as f32 / (ROAD_BOTTOM - ROAD_TOP) as f32).clamp(0.0, 1.0);
    let top_half = ROAD_TOP_W as f32 / 2.0;
    let bottom_half = ROAD_BOTTOM_W as f32 / 2.0;
    (top_half + (bottom_half - top_half) * t) as i32
}

fn lane_x(lane: Lane, y: i32, width: i32) -> i32 {
    let half = road_half_width(y);
    ROAD_CENTER_X + lane.direction() * (half / 2) - width / 2
}

fn city_hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}

fn window_light(building: u32, window: u32, tick: u64) -> Option<[u8; 3]> {
    let seed = city_hash(building.wrapping_mul(97) ^ window.wrapping_mul(1_009));
    let normally_lit = seed % 100 < 42;
    let interval = 75 + seed as u64 % 210;
    let moment = city_hash(seed ^ (tick / interval) as u32);
    let lit = if moment % 11 == 0 {
        !normally_lit
    } else {
        normally_lit
    };
    lit.then_some(if seed & 1 == 0 { YELLOW } else { CYAN })
}

fn draw_building(
    buffer: &mut [u8],
    index: u32,
    x: i32,
    width: i32,
    height: i32,
    color: [u8; 3],
    tick: u64,
) {
    let top = ROAD_TOP - height;
    fill_rect(buffer, x, top, width, height, color);
    fill_rect(buffer, x + width - 2, top + 2, 2, height - 2, SHADOW);

    // Different rooftop shapes keep the silhouette from looking tiled.
    match index % 4 {
        0 => {
            fill_rect(buffer, x + width / 2, top - 6, 1, 6, CYAN_DARK);
            fill_rect(buffer, x + width / 2 - 1, top - 6, 3, 1, ROAD_EDGE);
        }
        1 => fill_rect(buffer, x + 4, top - 3, (width - 8).max(3), 3, color),
        2 => {
            fill_rect(buffer, x + 3, top - 2, width - 6, 2, color);
            fill_rect(buffer, x + width - 6, top - 5, 1, 5, CYAN_DARK);
        }
        _ => {
            fill_rect(buffer, x + 2, top - 2, width - 4, 2, color);
            fill_rect(buffer, x + 5, top - 4, width - 10, 2, color);
        }
    }

    let mut window_index = 0;
    for wy in (top + 4..ROAD_TOP - 3).step_by(5) {
        for wx in (x + 3..x + width - 3).step_by(5) {
            if let Some(light) = window_light(index, window_index, tick) {
                fill_rect(buffer, wx, wy, 2, 2, light);
            } else {
                fill_rect(buffer, wx, wy, 2, 1, SKY);
            }
            window_index += 1;
        }
    }
}

fn draw_background(buffer: &mut [u8], tick: u64) {
    clear(buffer, SKY);
    fill_rect(buffer, 0, 28, WIDTH as i32, ROAD_TOP - 28, SKY_GLOW);
    fill_rect(
        buffer,
        0,
        42,
        WIDTH as i32,
        ROAD_TOP - 42,
        [0x3A, 0x20, 0x4A],
    );

    // Slow twinkling stars, each with its own phase.
    for (index, &(x, y)) in [
        (18, 24),
        (42, 32),
        (115, 25),
        (205, 29),
        (278, 24),
        (307, 35),
    ]
    .iter()
    .enumerate()
    {
        let bright = (tick / (40 + index as u64 * 13) + index as u64) % 5 != 0;
        fill_rect(buffer, x, y, 1, 1, if bright { WHITE } else { CYAN_DARK });
    }

    let back = [
        (-5, 31, 20),
        (22, 25, 28),
        (48, 37, 19),
        (82, 27, 31),
        (106, 32, 23),
        (136, 24, 29),
        (158, 36, 20),
        (191, 25, 30),
        (214, 38, 21),
        (248, 27, 28),
        (272, 32, 19),
        (300, 25, 26),
    ];
    for (index, &(x, width, height)) in back.iter().enumerate() {
        let color = if index % 2 == 0 {
            BUILDING_BACK
        } else {
            BUILDING_MID
        };
        draw_building(buffer, index as u32, x, width, height, color, tick);
    }

    let front = [
        (-3, 23, 24),
        (23, 18, 17),
        (44, 27, 30),
        (74, 21, 21),
        (98, 30, 27),
        (132, 18, 19),
        (152, 25, 34),
        (181, 27, 23),
        (211, 18, 29),
        (232, 31, 20),
        (266, 23, 32),
        (292, 31, 25),
    ];
    for (index, &(x, width, height)) in front.iter().enumerate() {
        draw_building(
            buffer,
            index as u32 + 50,
            x,
            width,
            height,
            BUILDING_FRONT,
            tick,
        );
    }

    fill_rect(
        buffer,
        0,
        ROAD_TOP,
        WIDTH as i32,
        HEIGHT as i32 - ROAD_TOP,
        GROUND,
    );
}

fn draw_pine(buffer: &mut [u8], x: i32, ground_y: i32, size: i32) {
    let trunk_height = size;
    let canopy_height = size * 2;
    let trunk_width = (size / 3).max(1);
    fill_rect(
        buffer,
        x - trunk_width / 2,
        ground_y - trunk_height,
        trunk_width,
        trunk_height,
        TREE_TRUNK,
    );

    let top = ground_y - trunk_height - canopy_height;
    for row in 0..canopy_height {
        let width = 1 + row * size * 2 / canopy_height;
        let color = if row < canopy_height / 2 {
            TREE_LIGHT
        } else {
            TREE_DARK
        };
        fill_rect(buffer, x - width / 2, top + row, width, 1, color);
    }
    fill_rect(buffer, x - 1, top + size / 2, 1, canopy_height, TREE_LIGHT);
}

fn draw_bush(buffer: &mut [u8], x: i32, ground_y: i32, size: i32) {
    let width = size * 2 + 2;
    fill_rect(
        buffer,
        x - width / 2,
        ground_y - size,
        width,
        size,
        TREE_DARK,
    );
    fill_rect(
        buffer,
        x - size / 2,
        ground_y - size - size / 2,
        size,
        size / 2 + 1,
        TREE_LIGHT,
    );
    fill_rect(buffer, x - width / 2 + 2, ground_y - size + 1, 2, 1, CYAN);
}

fn draw_roadside_plants(buffer: &mut [u8], scroll: f32) {
    let travel = ROAD_BOTTOM - ROAD_TOP;
    let offset = scroll as i32 % travel;

    for (index, base_y) in (ROAD_TOP..ROAD_BOTTOM).step_by(29).enumerate() {
        for (side, stagger) in [(-1, 0), (1, 15)] {
            let y = ROAD_TOP + (base_y - ROAD_TOP + offset + stagger) % travel;
            let perspective = y - ROAD_TOP;
            let size = 2 + perspective * 9 / travel;
            let half = road_half_width(y);
            let x = ROAD_CENTER_X + side * (half + 7 + size / 2);

            if (index + usize::from(side > 0)) % 3 == 1 {
                draw_bush(buffer, x, y, size.max(2));
            } else {
                draw_pine(buffer, x, y, size.max(2));
            }
        }
    }
}

fn draw_road(buffer: &mut [u8], scroll: f32) {
    for y in ROAD_TOP..=ROAD_BOTTOM {
        let half = road_half_width(y);
        let left = ROAD_CENTER_X - half;
        let right = ROAD_CENTER_X + half;
        fill_rect(buffer, left, y, right - left, 1, ROAD);
        let edge_width = 1 + ((y - ROAD_TOP) * 2 / (ROAD_BOTTOM - ROAD_TOP));
        fill_rect(buffer, left - edge_width, y, edge_width, 1, ROAD_EDGE);
        fill_rect(buffer, right, y, edge_width, 1, ROAD_EDGE);

        // Dashes accelerate toward the player, reinforcing forward motion.
        let perspective_scroll = scroll * (0.25 + (y - ROAD_TOP) as f32 / 160.0);
        if ((y as f32 + perspective_scroll) / 13.0) as i32 % 2 == 0 {
            let dash_width = 1 + (y - ROAD_TOP) / 70;
            fill_rect(
                buffer,
                ROAD_CENTER_X - dash_width / 2,
                y,
                dash_width,
                1,
                YELLOW,
            );
        }
    }

    // Scrolling roadside reflectors.
    let offset = scroll as i32 % 28;
    for base_y in (ROAD_TOP..ROAD_BOTTOM).step_by(28) {
        let y = ROAD_TOP + (base_y - ROAD_TOP + offset) % (ROAD_BOTTOM - ROAD_TOP);
        let half = road_half_width(y);
        let size = 1 + (y - ROAD_TOP) / 55;
        fill_rect(buffer, ROAD_CENTER_X - half - 7, y, size, size + 1, CYAN);
        fill_rect(buffer, ROAD_CENTER_X + half + 5, y, size, size + 1, CYAN);
    }

    draw_roadside_plants(buffer, scroll);
}

fn draw_sprite(buffer: &mut [u8], sprite: &[&str], x: i32, y: i32, color: [u8; 3], scale: f32) {
    for (row, line) in sprite.iter().enumerate() {
        for (col, b) in line.as_bytes().iter().enumerate() {
            if *b == b'1' {
                let left = (col as f32 * scale).round() as i32;
                let top = (row as f32 * scale).round() as i32;
                let right = ((col + 1) as f32 * scale).round() as i32;
                let bottom = ((row + 1) as f32 * scale).round() as i32;
                fill_rect(
                    buffer,
                    x + left,
                    y + top,
                    (right - left).max(1),
                    (bottom - top).max(1),
                    color,
                );
            }
        }
    }
}

fn draw_car(buffer: &mut [u8], lane: Lane) {
    let x = lane_x(lane, CAR_Y, CAR_W);
    fill_rect(buffer, x + 4, CAR_Y + CAR_H - 2, CAR_W - 8, 5, SHADOW);
    draw_sprite(buffer, &CAR_SPRITE, x, CAR_Y, CYAN, SPRITE_SCALE as f32);
    fill_rect(buffer, x + 12, CAR_Y + 4, 8, 6, CYAN_DARK);
    fill_rect(buffer, x + 6, CAR_Y + 14, 6, 2, YELLOW);
    fill_rect(buffer, x + 20, CAR_Y + 14, 6, 2, YELLOW);
    fill_rect(buffer, x + 2, CAR_Y + 18, 6, 4, SHADOW);
    fill_rect(buffer, x + 24, CAR_Y + 18, 6, 4, SHADOW);
}

fn donkey_scale(y: f32) -> f32 {
    let progress = ((y - ROAD_TOP as f32) / (ROAD_BOTTOM - ROAD_TOP) as f32).clamp(0.0, 1.0);
    1.25 + progress * 1.25
}

fn scaled(value: i32, scale: f32) -> i32 {
    (value as f32 * scale).round() as i32
}

fn draw_donkey(buffer: &mut [u8], game: &Game) {
    let y = game.donkey_y.round() as i32;
    let scale = donkey_scale(game.donkey_y);
    let width = scaled(DONKEY_BASE_W, scale);
    let height = scaled(DONKEY_BASE_H, scale);
    let x = lane_x(game.donkey_lane, y, width);
    fill_rect(
        buffer,
        x + scaled(3, scale),
        y + height - scaled(1, scale),
        width - scaled(6, scale),
        scaled(2, scale),
        SHADOW,
    );
    draw_sprite(buffer, &DONKEY_SPRITE, x, y, DONKEY, scale);
    fill_rect(
        buffer,
        x + scaled(6, scale),
        y + scaled(3, scale),
        scaled(1, scale),
        scaled(1, scale),
        SHADOW,
    );
    fill_rect(
        buffer,
        x + scaled(10, scale),
        y + scaled(3, scale),
        scaled(1, scale),
        scaled(1, scale),
        SHADOW,
    );
    fill_rect(
        buffer,
        x + scaled(7, scale),
        y + scaled(6, scale),
        scaled(3, scale),
        scaled(1, scale),
        WHITE,
    );
}

fn collides(game: &Game) -> bool {
    if game.car_lane != game.donkey_lane {
        return false;
    }
    let donkey_top = game.donkey_y;
    let donkey_bottom = game.donkey_y + DONKEY_BASE_H as f32 * donkey_scale(game.donkey_y) - 1.0;
    let car_top = CAR_Y as f32;
    let car_bottom = (CAR_Y + CAR_H - 1) as f32;
    donkey_top <= car_bottom && donkey_bottom >= car_top
}

fn update_logic(game: &mut Game) -> Option<SoundEffect> {
    if !game.started || game.over {
        return None;
    }

    let step = game.donkey_speed() / FPS as f32;
    game.donkey_y += step;
    game.road_scroll = (game.road_scroll + step) % 364.0;

    if collides(game) {
        game.over = true;
        return Some(SoundEffect::Crash);
    }

    if game.donkey_y > ROAD_BOTTOM as f32 {
        game.score += 1;
        game.spawn_donkey();
        return Some(SoundEffect::Score);
    }

    None
}

fn draw(buffer: &mut [u8], game: &Game) {
    draw_background(buffer, game.city_tick);
    draw_road(buffer, game.road_scroll);
    draw_donkey(buffer, game);
    draw_car(buffer, game.car_lane);

    fill_rect(buffer, 0, 0, WIDTH as i32, 20, SHADOW);
    fill_rect(buffer, 0, 19, WIDTH as i32, 1, CYAN_DARK);
    draw_text(buffer, 8, 6, "DONKEY", WHITE, 1);
    draw_text(buffer, 56, 6, "//", ROAD_EDGE, 1);
    draw_text(buffer, 72, 6, "RUST RUN", CYAN, 1);
    draw_text(buffer, 168, 6, &format!("LV {:02}", game.level()), WHITE, 1);
    draw_text(
        buffer,
        216,
        6,
        &format!("SCORE {:05}", game.score),
        YELLOW,
        1,
    );

    if !game.started {
        fill_rect(buffer, 58, 78, 204, 48, SHADOW);
        fill_rect(buffer, 58, 78, 204, 2, ROAD_EDGE);
        draw_text(buffer, 88, 88, "DODGE THE DONKEYS", WHITE, 1);
        draw_text(buffer, 76, 106, "[SPACE] START  [M] SOUND", CYAN, 1);
    } else if game.over {
        fill_rect(buffer, 68, 76, 184, 56, SHADOW);
        fill_rect(buffer, 68, 76, 184, 3, ROAD_EDGE);
        draw_text(buffer, 112, 86, "CRASH!", ROAD_EDGE, 2);
        draw_text(buffer, 84, 114, "[R] RETRY   [Q] QUIT", WHITE, 1);
    } else {
        draw_text(
            buffer,
            8,
            188,
            if game.sound_on {
                "M: SOUND ON"
            } else {
                "M: SOUND OFF"
            },
            WHITE,
            1,
        );
    }
}

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let audio = sdl.audio()?;
    let sounds = SoundEngine::new(&audio)?;
    let video = sdl.video()?;
    let window = video
        .window(
            "DONKEY.BAS (Rust CGA Port)",
            (WIDTH as u32) * SCALE,
            (HEIGHT as u32) * SCALE,
        )
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;
    canvas
        .set_logical_size(WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;
    canvas.clear();
    canvas.present();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;

    let mut buffer = vec![0_u8; WIDTH * HEIGHT * 3];
    let mut game = Game::new(seed_now());
    let mut event_pump = sdl.event_pump()?;

    let mut last_frame = Instant::now();
    let mut accumulator = Duration::ZERO;
    let mut left_was_down = false;
    let mut right_was_down = false;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    repeat: false,
                    ..
                } => {
                    game.sound_on = !game.sound_on;
                    sounds.play(game.sound_on, SoundEffect::Toggle);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    repeat: false,
                    ..
                } if !game.started => {
                    game.started = true;
                    game.spawn_donkey();
                    sounds.play(game.sound_on, SoundEffect::Start);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } if game.over => {
                    game.reset(seed_now());
                    sounds.play(game.sound_on, SoundEffect::Start);
                }
                _ => {}
            }
        }

        let ks = event_pump.keyboard_state();
        let left_down = ks.is_scancode_pressed(Scancode::Left);
        let right_down = ks.is_scancode_pressed(Scancode::Right);
        if game.started && !game.over {
            if left_down && !left_was_down && game.car_lane != Lane::Left {
                game.car_lane = Lane::Left;
                sounds.play(game.sound_on, SoundEffect::Move(Lane::Left));
            }
            if right_down && !right_was_down && game.car_lane != Lane::Right {
                game.car_lane = Lane::Right;
                sounds.play(game.sound_on, SoundEffect::Move(Lane::Right));
            }
        }
        left_was_down = left_down;
        right_was_down = right_down;

        let now = Instant::now();
        accumulator += now.saturating_duration_since(last_frame);
        last_frame = now;
        while accumulator >= FRAME_TIME {
            game.city_tick = game.city_tick.wrapping_add(1);
            if let Some(effect) = update_logic(&mut game) {
                sounds.play(game.sound_on, effect);
            }
            accumulator -= FRAME_TIME;
        }

        draw(&mut buffer, &game);
        texture
            .update(
                Rect::new(0, 0, WIDTH as u32, HEIGHT as u32),
                &buffer,
                WIDTH * 3,
            )
            .map_err(|e| e.to_string())?;
        canvas.copy(&texture, None, None)?;
        canvas.present();

        std::thread::sleep(Duration::from_millis(1));
    }

    Ok(())
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
    fn difficulty_increases_and_speed_is_capped() {
        let mut game = Game::new(1);
        assert_eq!(game.level(), 1);
        assert_eq!(game.donkey_speed(), 55.0);

        game.score = 5;
        assert_eq!(game.level(), 2);
        assert_eq!(game.donkey_speed(), 75.0);

        game.score = 100;
        assert_eq!(game.donkey_speed(), 135.0);
    }
}
