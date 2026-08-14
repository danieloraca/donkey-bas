use std::io::{Write, stdout};
use std::time::{Duration, Instant, SystemTime};

use font8x8::{BASIC_FONTS, UnicodeFonts};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SCALE: u32 = 3;
const FPS: u64 = 60;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FPS);

const BLACK: [u8; 3] = [0x00, 0x00, 0x00];
const CYAN: [u8; 3] = [0x00, 0xAA, 0xAA];
const MAGENTA: [u8; 3] = [0xAA, 0x00, 0xAA];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];

const ROAD_TOP: i32 = 20;
const ROAD_BOTTOM: i32 = 190;
const ROAD_TOP_W: i32 = 120;
const ROAD_BOTTOM_W: i32 = 260;
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
    fn x(self) -> i32 {
        match self {
            Self::Left => ROAD_CENTER_X - 48 - CAR_W / 2,
            Self::Right => ROAD_CENTER_X + 48 - CAR_W / 2,
        }
    }
}

struct Game {
    car_lane: Lane,
    donkey_lane: Lane,
    donkey_y: i32,
    score: u32,
    over: bool,
    started: bool,
    sound_on: bool,
    rng: u32,
    move_frame_counter: u32,
}

impl Game {
    fn new(seed: u32) -> Self {
        Self {
            car_lane: Lane::Left,
            donkey_lane: Lane::Right,
            donkey_y: -DONKEY_H,
            score: 0,
            over: false,
            started: false,
            sound_on: true,
            rng: seed.max(1),
            move_frame_counter: 0,
        }
    }

    fn reset(&mut self, seed: u32) {
        *self = Self::new(seed);
        self.started = true;
        self.spawn_donkey();
    }

    fn next_u32(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        self.rng
    }

    fn spawn_donkey(&mut self) {
        self.donkey_lane = if self.next_u32() & 1 == 0 {
            Lane::Left
        } else {
            Lane::Right
        };
        self.donkey_y = -DONKEY_H;
        self.move_frame_counter = 0;
    }

    fn donkey_step_frames(&self) -> u32 {
        let speedup = self.score / 5;
        6_u32.saturating_sub(speedup).max(2)
    }
}

fn seed_now() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0xC0FFEE_u32)
}

fn beep(sound_on: bool, count: u8) {
    if !sound_on {
        return;
    }
    let mut out = stdout();
    for _ in 0..count {
        let _ = out.write_all(b"\x07");
    }
    let _ = out.flush();
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
    for py in y.max(0)..(y + h).min(HEIGHT as i32) {
        for px in x.max(0)..(x + w).min(WIDTH as i32) {
            put_pixel(buffer, px, py, color);
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
    let t = (y - ROAD_TOP) as f32 / (ROAD_BOTTOM - ROAD_TOP) as f32;
    let top_half = ROAD_TOP_W as f32 / 2.0;
    let bottom_half = ROAD_BOTTOM_W as f32 / 2.0;
    (top_half + (bottom_half - top_half) * t) as i32
}

fn draw_road(buffer: &mut [u8]) {
    for y in ROAD_TOP..=ROAD_BOTTOM {
        let half = road_half_width(y);
        let left = ROAD_CENTER_X - half;
        let right = ROAD_CENTER_X + half;
        fill_rect(buffer, left, y, right - left, 1, BLACK);
        fill_rect(buffer, left - 2, y, 2, 1, CYAN);
        fill_rect(buffer, right, y, 2, 1, CYAN);

        let dash_period = 14;
        if ((y + 4) / dash_period) % 2 == 0 {
            let lane_sep = (half * 48) / (ROAD_BOTTOM_W / 2);
            let cx_l = ROAD_CENTER_X - lane_sep / 2;
            let cx_r = ROAD_CENTER_X + lane_sep / 2;
            fill_rect(buffer, cx_l - 1, y, 2, 1, MAGENTA);
            fill_rect(buffer, cx_r - 1, y, 2, 1, MAGENTA);
        }
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

fn collides(game: &Game) -> bool {
    if game.car_lane != game.donkey_lane {
        return false;
    }
    let donkey_top = game.donkey_y;
    let donkey_bottom = game.donkey_y + DONKEY_H - 1;
    let car_top = CAR_Y;
    let car_bottom = CAR_Y + CAR_H - 1;
    donkey_top <= car_bottom && donkey_bottom >= car_top
}

fn update_logic(game: &mut Game) {
    if !game.started || game.over {
        return;
    }

    game.move_frame_counter += 1;
    if game.move_frame_counter < game.donkey_step_frames() {
        return;
    }
    game.move_frame_counter = 0;
    game.donkey_y += 2;

    if collides(game) {
        game.over = true;
        beep(game.sound_on, 3);
        return;
    }

    if game.donkey_y > ROAD_BOTTOM {
        game.score += 1;
        beep(game.sound_on, 1);
        game.spawn_donkey();
    }
}

fn draw(buffer: &mut [u8], game: &Game) {
    clear(buffer, MAGENTA);
    draw_road(buffer);

    let donkey_x = game.donkey_lane.x();
    draw_sprite(buffer, &DONKEY_SPRITE, donkey_x, game.donkey_y, WHITE);

    let car_x = game.car_lane.x();
    draw_sprite(buffer, &CAR_SPRITE, car_x, CAR_Y, CYAN);

    draw_text(buffer, 8, 4, "DONKEY.BAS", WHITE, 1);
    draw_text(
        buffer,
        160,
        4,
        &format!("SCORE {:05}", game.score),
        WHITE,
        1,
    );
    draw_text(
        buffer,
        8,
        14,
        &format!("SOUND {}", if game.sound_on { "ON " } else { "OFF" }),
        WHITE,
        1,
    );

    if !game.started {
        draw_text(buffer, 56, 92, "PRESS SPACE TO START", WHITE, 1);
    } else if game.over {
        draw_text(buffer, 68, 84, "CRASH!", WHITE, 2);
        draw_text(buffer, 50, 104, "R TO RESTART  Q TO QUIT", WHITE, 1);
    }
}

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
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
                    beep(game.sound_on, 1);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    repeat: false,
                    ..
                } if !game.started => {
                    game.started = true;
                    game.spawn_donkey();
                    beep(game.sound_on, 1);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } if game.over => {
                    game.reset(seed_now());
                    beep(game.sound_on, 1);
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
                beep(game.sound_on, 1);
            }
            if right_down && !right_was_down && game.car_lane != Lane::Right {
                game.car_lane = Lane::Right;
                beep(game.sound_on, 1);
            }
        }
        left_was_down = left_down;
        right_was_down = right_down;

        let now = Instant::now();
        accumulator += now.saturating_duration_since(last_frame);
        last_frame = now;
        while accumulator >= FRAME_TIME {
            update_logic(&mut game);
            accumulator -= FRAME_TIME;
        }

        draw(&mut buffer, &game);
        texture
            .update(Rect::new(0, 0, WIDTH as u32, HEIGHT as u32), &buffer, WIDTH * 3)
            .map_err(|e| e.to_string())?;
        canvas.copy(&texture, None, None)?;
        canvas.present();

        std::thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}
