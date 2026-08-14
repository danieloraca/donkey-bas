use std::cell::Cell;

use sdl2::AudioSubsystem;
use sdl2::audio::{AudioQueue, AudioSpecDesired};

use crate::game::Lane;

#[derive(Clone, Copy)]
pub(crate) enum SoundEffect {
    Start,
    Countdown,
    Go,
    Pause,
    Resume,
    Move(Lane),
    Score,
    NearMiss,
    Hit,
    Crash,
    Toggle,
}

pub(crate) struct SoundEngine {
    queue: AudioQueue<f32>,
    sample_rate: f32,
    music_step: Cell<u64>,
    music_running: Cell<bool>,
}

impl SoundEngine {
    pub(crate) fn new(audio: &AudioSubsystem) -> Result<Self, String> {
        let desired = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1),
            samples: Some(512),
        };
        let queue = audio.open_queue::<f32, _>(None, &desired)?;
        let sample_rate = queue.spec().freq as f32;
        queue.resume();
        Ok(Self {
            queue,
            sample_rate,
            music_step: Cell::new(0),
            music_running: Cell::new(false),
        })
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

    pub(crate) fn play(&self, enabled: bool, effect: SoundEffect) {
        if !enabled {
            return;
        }

        self.music_running.set(false);
        self.queue.clear();
        let mut samples = Vec::new();
        match effect {
            SoundEffect::Start => {
                for frequency in [220.0, 330.0, 440.0, 660.0] {
                    self.tone(&mut samples, frequency, 55, 0.16);
                    self.rest(&mut samples, 12);
                }
            }
            SoundEffect::Countdown => self.tone(&mut samples, 440.0, 90, 0.15),
            SoundEffect::Go => {
                self.tone(&mut samples, 660.0, 65, 0.16);
                self.tone(&mut samples, 880.0, 110, 0.18);
            }
            SoundEffect::Pause => {
                self.tone(&mut samples, 440.0, 55, 0.12);
                self.tone(&mut samples, 330.0, 90, 0.12);
            }
            SoundEffect::Resume => {
                self.tone(&mut samples, 330.0, 55, 0.12);
                self.tone(&mut samples, 440.0, 90, 0.12);
            }
            SoundEffect::Move(lane) => {
                let frequency = if lane == Lane::Left { 330.0 } else { 440.0 };
                self.tone(&mut samples, frequency, 32, 0.09);
            }
            SoundEffect::Score => {
                self.tone(&mut samples, 660.0, 55, 0.14);
                self.tone(&mut samples, 880.0, 90, 0.16);
            }
            SoundEffect::NearMiss => {
                for frequency in [740.0, 988.0, 1_318.0] {
                    self.tone(&mut samples, frequency, 45, 0.15);
                }
            }
            SoundEffect::Hit => {
                self.tone(&mut samples, 180.0, 70, 0.17);
                self.noise(&mut samples, 110, 0.12);
            }
            SoundEffect::Crash => {
                self.tone(&mut samples, 140.0, 90, 0.18);
                self.noise(&mut samples, 280, 0.20);
            }
            SoundEffect::Toggle => self.tone(&mut samples, 520.0, 70, 0.12),
        }

        let _ = self.queue.queue_audio(&samples);
    }

    pub(crate) fn update_music(&self, enabled: bool, active: bool, level: u32) {
        if !enabled || !active {
            if self.music_running.replace(false) {
                self.queue.clear();
            }
            return;
        }

        self.music_running.set(true);
        let queued_samples = self.queue.size() as usize / std::mem::size_of::<f32>();
        let refill_at = (self.sample_rate * 0.10) as usize;
        if queued_samples > refill_at {
            return;
        }

        let step = self.music_step.get();
        let chunk = self.music_chunk(level, step);
        if self.queue.queue_audio(&chunk).is_ok() {
            self.music_step.set(step.wrapping_add(1));
        }
    }

    fn music_chunk(&self, level: u32, step: u64) -> Vec<f32> {
        let bpm = music_bpm(level) as f32;
        let duration = 60.0 / bpm / 4.0;
        let count = (self.sample_rate * duration) as usize;
        let arpeggio = [261.63, 311.13, 392.0, 466.16, 392.0, 311.13, 261.63, 392.0];
        let bass_line = [130.81, 116.54, 103.83, 116.54];
        let lead_frequency = arpeggio[step as usize % arpeggio.len()];
        let bass_frequency = bass_line[(step as usize / 8) % bass_line.len()];
        let mut lfsr = 0xACE1_u16 ^ step as u16;
        let mut output = Vec::with_capacity(count);

        for index in 0..count {
            let time = index as f32 / self.sample_rate;
            let position = index as f32 / count as f32;
            let gate = if position < 0.72 { 1.0 } else { 0.0 };
            let lead = square_wave(lead_frequency, time) * 0.045 * gate;
            let bass = triangle_wave(bass_frequency, time) * 0.065;
            let harmony = if level >= 5 {
                square_wave(lead_frequency * 1.5, time) * 0.025 * gate
            } else {
                0.0
            };

            let kick = if step % 4 == 0 && position < 0.32 {
                let envelope = 1.0 - position / 0.32;
                (std::f32::consts::TAU * time * (75.0 - position * 70.0)).sin() * 0.11 * envelope
            } else {
                0.0
            };

            let hat = if level >= 3 && step % 2 == 1 && position < 0.18 {
                let bit = (lfsr ^ (lfsr >> 1)) & 1;
                lfsr = (lfsr >> 1) | (bit << 15);
                let noise = if lfsr & 1 == 0 { -1.0 } else { 1.0 };
                noise * 0.035 * (1.0 - position / 0.18)
            } else {
                0.0
            };

            let release = ((count - index) as f32 / 48.0).min(1.0);
            output.push(((lead + bass + harmony + kick + hat) * release).clamp(-0.35, 0.35));
        }
        output
    }
}

fn square_wave(frequency: f32, time: f32) -> f32 {
    if (time * frequency).fract() < 0.5 {
        1.0
    } else {
        -1.0
    }
}

fn triangle_wave(frequency: f32, time: f32) -> f32 {
    1.0 - 4.0 * ((time * frequency).fract() - 0.5).abs()
}

fn music_bpm(level: u32) -> u32 {
    108 + level.saturating_sub(1).min(30) * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_tempo_rises_gently_and_is_bounded() {
        assert_eq!(music_bpm(1), 108);
        assert!(music_bpm(10) > music_bpm(1));
        assert_eq!(music_bpm(100), 168);
    }
}
