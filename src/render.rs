use font8x8::{BASIC_FONTS, UnicodeFonts};

use crate::game::{
    CAR_H, CAR_W, CAR_Y, DONKEY_BASE_H, DONKEY_BASE_W, Game, HEIGHT, ObstaclePattern, ROAD_BOTTOM,
    ROAD_CENTER_X, ROAD_TOP, SPRITE_SCALE, WIDTH, donkey_scale, road_center, road_half_width,
    road_position_x, scaled,
};

pub(crate) const SCALE: u32 = 3;

const SKY: [u8; 3] = [0x08, 0x0D, 0x22];
const SKY_GLOW: [u8; 3] = [0x2C, 0x1E, 0x4E];
const GROUND: [u8; 3] = [0x0B, 0x34, 0x2D];
const GROUND_ALT: [u8; 3] = [0x0D, 0x3D, 0x34];
const ROAD: [u8; 3] = [0x20, 0x24, 0x32];
const ROAD_ALT: [u8; 3] = [0x25, 0x29, 0x38];
const ROAD_EDGE: [u8; 3] = [0xFF, 0x4F, 0x81];
const SUN: [u8; 3] = [0xFF, 0x78, 0x69];
const CYAN: [u8; 3] = [0x25, 0xE6, 0xD2];
const CYAN_DARK: [u8; 3] = [0x08, 0x61, 0x70];
const YELLOW: [u8; 3] = [0xFF, 0xD1, 0x66];
const DONKEY: [u8; 3] = [0xD8, 0x91, 0x56];
const DONKEY_CROSSING: [u8; 3] = [0xF0, 0x62, 0x7A];
const SHADOW: [u8; 3] = [0x08, 0x0A, 0x12];
const BUILDING_BACK: [u8; 3] = [0x12, 0x18, 0x38];
const BUILDING_MID: [u8; 3] = [0x1A, 0x20, 0x4A];
const BUILDING_FRONT: [u8; 3] = [0x13, 0x19, 0x31];
const TREE_DARK: [u8; 3] = [0x08, 0x45, 0x3C];
const TREE_LIGHT: [u8; 3] = [0x10, 0x78, 0x58];
const TREE_TRUNK: [u8; 3] = [0x73, 0x43, 0x35];
const WHITE: [u8; 3] = [0xF4, 0xF7, 0xFF];
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

    let sun_x = ROAD_CENTER_X;
    for dy in -13..=13 {
        let half = (((13 * 13 - dy * dy) as f32).sqrt()) as i32;
        if (dy + 13) % 4 != 2 {
            fill_rect(buffer, sun_x - half, 35 + dy, half * 2, 1, SUN);
        }
    }

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
            let x = road_center(y, scroll) + side * (half + 7 + size / 2);

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
        let depth = (y - ROAD_TOP) as f32 / (ROAD_BOTTOM - ROAD_TOP) as f32;
        let half = road_half_width(y);
        let center = road_center(y, scroll);
        let left = center - half;
        let right = center + half;
        let perspective_scroll = scroll * (0.18 + depth * 1.35);
        let segment = ((y as f32 + perspective_scroll) / 13.0) as i32;
        let ground_color = if segment % 2 == 0 { GROUND } else { GROUND_ALT };
        let road_color = if segment % 2 == 0 { ROAD } else { ROAD_ALT };
        fill_rect(buffer, 0, y, WIDTH as i32, 1, ground_color);
        fill_rect(buffer, left, y, right - left, 1, road_color);

        let edge_width = 1 + ((y - ROAD_TOP) * 3 / (ROAD_BOTTOM - ROAD_TOP));
        let edge_color = if segment % 2 == 0 { WHITE } else { ROAD_EDGE };
        fill_rect(buffer, left, y, edge_width, 1, edge_color);
        fill_rect(buffer, right - edge_width, y, edge_width, 1, edge_color);
    }

    draw_center_markings(buffer, scroll);

    // Scrolling roadside reflectors.
    let offset = scroll as i32 % 28;
    for base_y in (ROAD_TOP..ROAD_BOTTOM).step_by(28) {
        let y = ROAD_TOP + (base_y - ROAD_TOP + offset) % (ROAD_BOTTOM - ROAD_TOP);
        let half = road_half_width(y);
        let center = road_center(y, scroll);
        let size = 1 + (y - ROAD_TOP) / 55;
        fill_rect(buffer, center - half - 7, y, size, size + 1, CYAN);
        fill_rect(buffer, center + half + 5, y, size, size + 1, CYAN);
    }

    draw_roadside_plants(buffer, scroll);
}

fn project_road_depth(depth: f32) -> i32 {
    ROAD_TOP + (depth.clamp(0.0, 1.0).powf(1.65) * (ROAD_BOTTOM - ROAD_TOP) as f32) as i32
}

fn draw_center_markings(buffer: &mut [u8], scroll: f32) {
    let motion = (scroll / 182.0).fract();
    for index in 0..8 {
        let start = (index as f32 / 8.0 + motion).fract();
        let end = (start + 0.045 + start * 0.025).min(1.0);
        let top = project_road_depth(start);
        let bottom = project_road_depth(end).max(top + 1);

        for y in top..bottom {
            let depth = (y - ROAD_TOP) as f32 / (ROAD_BOTTOM - ROAD_TOP) as f32;
            let width = 1 + (depth * 3.0) as i32;
            let center = road_center(y, scroll);
            fill_rect(buffer, center - width / 2, y, width, 1, YELLOW);
        }
    }
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

fn draw_car(buffer: &mut [u8], game: &Game) {
    let mut x = road_position_x(game.car_position, CAR_Y, CAR_W, game.road_scroll);
    let crash_age = game.city_tick.saturating_sub(game.crash_started_at);
    if game.over && crash_age < 18 {
        x += (city_hash(game.city_tick as u32) % 5) as i32 - 2;
    }
    fill_rect(buffer, x + 4, CAR_Y + CAR_H - 2, CAR_W - 8, 5, SHADOW);
    draw_sprite(buffer, &CAR_SPRITE, x, CAR_Y, CYAN, SPRITE_SCALE as f32);
    fill_rect(buffer, x + 12, CAR_Y + 4, 8, 6, CYAN_DARK);
    fill_rect(buffer, x + 6, CAR_Y + 14, 6, 2, YELLOW);
    fill_rect(buffer, x + 20, CAR_Y + 14, 6, 2, YELLOW);
    fill_rect(buffer, x + 2, CAR_Y + 18, 6, 4, SHADOW);
    fill_rect(buffer, x + 24, CAR_Y + 18, 6, 4, SHADOW);
}

fn draw_crash_effect(buffer: &mut [u8], game: &Game) {
    if !game.over {
        return;
    }
    let age = game.city_tick.saturating_sub(game.crash_started_at);
    if age > 45 {
        return;
    }

    let car_x = road_position_x(game.car_position, CAR_Y, CAR_W, game.road_scroll);
    let origin_x = car_x + CAR_W / 2;
    let origin_y = CAR_Y + CAR_H / 2;
    for index in 0..18_u32 {
        let seed = city_hash(index.wrapping_mul(977) + 31);
        let angle = seed as f32 / u32::MAX as f32 * std::f32::consts::TAU;
        let speed = 0.35 + (seed >> 16) as f32 / u16::MAX as f32 * 0.75;
        let distance = age as f32 * speed;
        let x = origin_x + (angle.cos() * distance) as i32;
        let y = origin_y + (angle.sin() * distance + age as f32 * age as f32 * 0.012) as i32;
        let color = match index % 3 {
            0 => YELLOW,
            1 => ROAD_EDGE,
            _ => WHITE,
        };
        let size = if age < 14 { 2 } else { 1 };
        fill_rect(buffer, x, y, size, size, color);
    }

    if age < 12 && age % 4 < 2 {
        fill_rect(buffer, 0, 20, WIDTH as i32, 2, WHITE);
        fill_rect(buffer, 0, HEIGHT as i32 - 2, WIDTH as i32, 2, WHITE);
        fill_rect(buffer, 0, 20, 2, HEIGHT as i32 - 20, WHITE);
        fill_rect(buffer, WIDTH as i32 - 2, 20, 2, HEIGHT as i32 - 20, WHITE);
    }
}

fn draw_donkey(buffer: &mut [u8], game: &Game) {
    let y = game.donkey_y.round() as i32;
    let scale = donkey_scale(game.donkey_y);
    let width = scaled(DONKEY_BASE_W, scale);
    let height = scaled(DONKEY_BASE_H, scale);
    let x = road_position_x(game.donkey_position, y, width, game.road_scroll);
    let body_color = if game.obstacle_pattern == ObstaclePattern::Crossing {
        DONKEY_CROSSING
    } else {
        DONKEY
    };
    fill_rect(
        buffer,
        x + scaled(3, scale),
        y + height - scaled(1, scale),
        width - scaled(6, scale),
        scaled(2, scale),
        SHADOW,
    );
    draw_sprite(buffer, &DONKEY_SPRITE, x, y, body_color, scale);
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

pub(crate) fn draw(buffer: &mut [u8], game: &Game) {
    draw_background(buffer, game.city_tick);
    draw_road(buffer, game.road_scroll);
    draw_donkey(buffer, game);
    draw_car(buffer, game);
    draw_crash_effect(buffer, game);

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
        fill_rect(buffer, 58, 70, 204, 66, SHADOW);
        fill_rect(buffer, 58, 70, 204, 2, ROAD_EDGE);
        draw_text(buffer, 88, 82, "DODGE THE DONKEYS", WHITE, 1);
        draw_text(buffer, 76, 100, "[SPACE] START  [M] SOUND", CYAN, 1);
        draw_text(
            buffer,
            120,
            118,
            &format!("BEST {:05}", game.best_score),
            YELLOW,
            1,
        );
    } else if game.over {
        fill_rect(buffer, 68, 68, 184, 76, SHADOW);
        fill_rect(buffer, 68, 68, 184, 3, ROAD_EDGE);
        draw_text(buffer, 112, 78, "CRASH!", ROAD_EDGE, 2);
        if game.new_high_score {
            draw_text(buffer, 104, 104, "NEW HIGH SCORE", YELLOW, 1);
        } else {
            draw_text(
                buffer,
                120,
                104,
                &format!("BEST {:05}", game.best_score),
                YELLOW,
                1,
            );
        }
        draw_text(buffer, 84, 126, "[R] RETRY   [Q] QUIT", WHITE, 1);
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
        draw_text(
            buffer,
            240,
            188,
            &format!("HI {:05}", game.best_score),
            YELLOW,
            1,
        );
    }
}
