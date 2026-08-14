mod audio;
mod game;
mod render;
mod score;

use std::time::{Duration, Instant};

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::video::FullscreenType;

use audio::{SoundEffect, SoundEngine};
use game::{FPS, Game, GameEvent, HEIGHT, Lane, WIDTH, seed_now};
use render::{SCALE, draw};
use score::ScoreStore;

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FPS);

fn steer(game: &mut Game, sounds: &SoundEngine, lane: Lane) {
    if game.started && !game.over && !game.paused && game.car_lane != lane {
        game.car_lane = lane;
        sounds.play(game.sound_on, SoundEffect::Move(lane));
    }
}

fn toggle_pause(game: &mut Game, sounds: &SoundEngine) {
    if !game.started || game.over {
        return;
    }
    game.paused = !game.paused;
    sounds.play(
        game.sound_on,
        if game.paused {
            SoundEffect::Pause
        } else {
            SoundEffect::Resume
        },
    );
}

fn start_or_restart(game: &mut Game, sounds: &SoundEngine) {
    if !game.started {
        game.start();
        sounds.play(game.sound_on, SoundEffect::Start);
    } else if game.over {
        game.reset(seed_now());
        sounds.play(game.sound_on, SoundEffect::Start);
    }
}

fn open_controller(
    subsystem: &sdl2::GameControllerSubsystem,
    controllers: &mut Vec<GameController>,
    index: u32,
) {
    if subsystem.is_game_controller(index) {
        if let Ok(controller) = subsystem.open(index) {
            controllers.push(controller);
        }
    }
}

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let audio = sdl.audio()?;
    let sounds = SoundEngine::new(&audio)?;
    let controller_subsystem = sdl.game_controller()?;
    let mut controllers = Vec::new();
    if let Ok(count) = controller_subsystem.num_joysticks() {
        for index in 0..count {
            open_controller(&controller_subsystem, &mut controllers, index);
        }
    }
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
    let mut fullscreen = false;

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
                    keycode: Some(Keycode::P),
                    repeat: false,
                    ..
                } => toggle_pause(&mut game, &sounds),
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    repeat: false,
                    ..
                } => {
                    let target = if fullscreen {
                        FullscreenType::Off
                    } else {
                        FullscreenType::Desktop
                    };
                    if canvas.window_mut().set_fullscreen(target).is_ok() {
                        fullscreen = !fullscreen;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    repeat: false,
                    ..
                } if !game.started => start_or_restart(&mut game, &sounds),
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } if game.over => start_or_restart(&mut game, &sounds),
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } if game.started && !game.over && !game.paused => {
                    toggle_pause(&mut game, &sounds);
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    open_controller(&controller_subsystem, &mut controllers, which);
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    controllers.retain(|controller| controller.instance_id() != which);
                }
                Event::ControllerAxisMotion {
                    axis: Axis::LeftX,
                    value,
                    ..
                } if value < -16_000 => steer(&mut game, &sounds, Lane::Left),
                Event::ControllerAxisMotion {
                    axis: Axis::LeftX,
                    value,
                    ..
                } if value > 16_000 => steer(&mut game, &sounds, Lane::Right),
                Event::ControllerButtonDown { button, .. } => match button {
                    Button::DPadLeft => steer(&mut game, &sounds, Lane::Left),
                    Button::DPadRight => steer(&mut game, &sounds, Lane::Right),
                    Button::A => start_or_restart(&mut game, &sounds),
                    Button::Start | Button::Back => {
                        if !game.started || game.over {
                            start_or_restart(&mut game, &sounds);
                        } else {
                            toggle_pause(&mut game, &sounds);
                        }
                    }
                    Button::Y => {
                        game.sound_on = !game.sound_on;
                        sounds.play(game.sound_on, SoundEffect::Toggle);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        let keyboard = event_pump.keyboard_state();
        let left_down = keyboard.is_scancode_pressed(Scancode::Left);
        let right_down = keyboard.is_scancode_pressed(Scancode::Right);
        if game.started && !game.over && !game.paused {
            if left_down && !left_was_down {
                steer(&mut game, &sounds, Lane::Left);
            }
            if right_down && !right_was_down {
                steer(&mut game, &sounds, Lane::Right);
            }
        }
        left_was_down = left_down;
        right_was_down = right_down;

        let now = Instant::now();
        accumulator += now.saturating_duration_since(last_frame);
        last_frame = now;
        while accumulator >= FRAME_TIME {
            if !game.paused {
                game.city_tick = game.city_tick.wrapping_add(1);
                game.update_ambient_motion();
            }
            if let Some(event) = game.update() {
                if matches!(event, GameEvent::Score | GameEvent::NearMiss) && game.new_high_score {
                    if let Err(error) = score_store.save(game.best_score) {
                        eprintln!("could not save high score: {error}");
                    }
                }
                let effect = match event {
                    GameEvent::Countdown => SoundEffect::Countdown,
                    GameEvent::Go => SoundEffect::Go,
                    GameEvent::CrossingWarning => SoundEffect::CrossingWarning,
                    GameEvent::Score => SoundEffect::Score,
                    GameEvent::NearMiss => SoundEffect::NearMiss,
                    GameEvent::Hit => SoundEffect::Hit,
                    GameEvent::Crash => SoundEffect::Crash,
                };
                sounds.play(game.sound_on, effect);
            }
            accumulator -= FRAME_TIME;
        }

        sounds.update_music(
            game.sound_on,
            game.started && !game.over && !game.paused,
            game.level(),
        );

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
