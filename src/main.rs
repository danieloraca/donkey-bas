mod audio;
mod game;
mod render;
mod score;

use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

use audio::{SoundEffect, SoundEngine};
use game::{FPS, Game, GameEvent, HEIGHT, Lane, WIDTH, seed_now};
use render::{SCALE, draw};
use score::ScoreStore;

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FPS);

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
        .map_err(|error| error.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|error| error.to_string())?;
    canvas
        .set_logical_size(WIDTH as u32, HEIGHT as u32)
        .map_err(|error| error.to_string())?;
    canvas.clear();
    canvas.present();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
        .map_err(|error| error.to_string())?;

    let mut buffer = vec![0_u8; WIDTH * HEIGHT * 3];
    let score_store = ScoreStore::new();
    let mut game = Game::new(seed_now());
    game.best_score = score_store.load();
    let mut event_pump = sdl.event_pump()?;

    let mut last_frame = Instant::now();
    let mut accumulator = Duration::ZERO;
    let mut left_was_down = false;
    let mut right_was_down = false;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape | Keycode::Q),
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

        let keyboard = event_pump.keyboard_state();
        let left_down = keyboard.is_scancode_pressed(Scancode::Left);
        let right_down = keyboard.is_scancode_pressed(Scancode::Right);
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
            game.update_ambient_motion();
            if let Some(event) = game.update() {
                if matches!(event, GameEvent::Score | GameEvent::NearMiss) && game.new_high_score {
                    if let Err(error) = score_store.save(game.best_score) {
                        eprintln!("could not save high score: {error}");
                    }
                }
                let effect = match event {
                    GameEvent::Score => SoundEffect::Score,
                    GameEvent::NearMiss => SoundEffect::NearMiss,
                    GameEvent::Hit => SoundEffect::Hit,
                    GameEvent::Crash => SoundEffect::Crash,
                };
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
            .map_err(|error| error.to_string())?;
        canvas.copy(&texture, None, None)?;
        canvas.present();

        std::thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}
