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
const WHITE: [u8; 3] = [0xF4, 0xF7, 0xFF];

const ROAD_TOP: i32 = 40;
const ROAD_BOTTOM: i32 = 199;
const ROAD_TOP_W: i32 = 72;
const ROAD_BOTTOM_W: i32 = 278;
const ROAD_CENTER_X: i32 = (WIDTH as i32) / 2;

const CAR_W: i32 = 16;
const CAR_H: i32 = 12;
const DONKEY_H: i32 = 12;
const CAR_Y: i32 = 162;

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
            score: 0,
            over: false,
            started: false,
            sound_on: true,
            rng: seed.max(1),
        }
    }

    fn reset(&mut self, seed: u32) {
        *self = Self::new(seed);
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

fn put_pixel(buffer: &mut [u8], x: i32, y: i32, color: [u8; 3]) {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
        return;
    }
    let idx = ((y as usize) * WIDTH + (x as usize)) * 3;
    buffer[idx..idx + 3].copy_from_slice(&color);
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

fn draw_background(buffer: &mut [u8]) {
    clear(buffer, SKY);
    fill_rect(buffer, 0, 26, WIDTH as i32, 14, SKY_GLOW);

    // A tiny neon skyline makes the horizon feel less empty.
    for &(x, w, h) in &[
        (0, 24, 13),
        (28, 18, 8),
        (50, 30, 16),
        (84, 15, 10),
        (224, 18, 9),
        (247, 27, 15),
        (279, 18, 11),
        (301, 19, 17),
    ] {
        fill_rect(buffer, x, ROAD_TOP - h, w, h, SHADOW);
        if w > 20 {
            fill_rect(buffer, x + 5, ROAD_TOP - h + 4, 2, 2, YELLOW);
            fill_rect(buffer, x + 13, ROAD_TOP - h + 4, 2, 2, CYAN);
        }
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
}

fn draw_sprite(buffer: &mut [u8], sprite: &[&str], x: i32, y: i32, color: [u8; 3]) {
    for (row, line) in sprite.iter().enumerate() {
        for (col, b) in line.as_bytes().iter().enumerate() {
            if *b == b'1' {
                put_pixel(buffer, x + col as i32, y + row as i32, color);
            }
        }
    }
}

fn draw_car(buffer: &mut [u8], lane: Lane) {
    let x = lane_x(lane, CAR_Y, CAR_W);
    fill_rect(buffer, x + 2, CAR_Y + CAR_H - 1, CAR_W - 4, 3, SHADOW);
    draw_sprite(buffer, &CAR_SPRITE, x, CAR_Y, CYAN);
    fill_rect(buffer, x + 6, CAR_Y + 2, 4, 3, CYAN_DARK);
    fill_rect(buffer, x + 3, CAR_Y + 7, 3, 1, YELLOW);
    fill_rect(buffer, x + 10, CAR_Y + 7, 3, 1, YELLOW);
    fill_rect(buffer, x + 1, CAR_Y + 9, 3, 2, SHADOW);
    fill_rect(buffer, x + 12, CAR_Y + 9, 3, 2, SHADOW);
}

fn draw_donkey(buffer: &mut [u8], game: &Game) {
    let y = game.donkey_y.round() as i32;
    let x = lane_x(game.donkey_lane, y, CAR_W);
    fill_rect(buffer, x + 3, y + DONKEY_H - 1, CAR_W - 6, 2, SHADOW);
    draw_sprite(buffer, &DONKEY_SPRITE, x, y, DONKEY);
    put_pixel(buffer, x + 6, y + 3, SHADOW);
    put_pixel(buffer, x + 10, y + 3, SHADOW);
    fill_rect(buffer, x + 7, y + 6, 3, 1, WHITE);
}

fn collides(game: &Game) -> bool {
    if game.car_lane != game.donkey_lane {
        return false;
    }
    let donkey_top = game.donkey_y;
    let donkey_bottom = game.donkey_y + DONKEY_H as f32 - 1.0;
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
    draw_background(buffer);
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
