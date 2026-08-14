use sdl2::AudioSubsystem;
use sdl2::audio::{AudioQueue, AudioSpecDesired};

use crate::game::Lane;

#[derive(Clone, Copy)]
pub(crate) enum SoundEffect {
    Start,
    Move(Lane),
    Score,
    Crash,
    Toggle,
}

pub(crate) struct SoundEngine {
    queue: AudioQueue<f32>,
    sample_rate: f32,
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

    pub(crate) fn play(&self, enabled: bool, effect: SoundEffect) {
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
