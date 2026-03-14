//! Toile Engine — Audio Demo (Week 4)
//!
//! Space: play beep SFX
//! M: toggle music pause/resume
//! Up/Down arrows: master volume
//!
//! Run with: `cargo run --example audio_demo`

use std::path::Path;

use toile_app::{App, Game, GameContext, Key, PlaybackId, SoundId};

struct AudioDemo {
    beep: Option<SoundId>,
    music_playback: Option<PlaybackId>,
    music_paused: bool,
}

impl Game for AudioDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        self.beep = Some(
            ctx.audio
                .load_sound(Path::new("assets/beep.wav"))
                .expect("Failed to load beep.wav"),
        );

        // Load and loop background music
        let music_id = ctx
            .audio
            .load_sound(Path::new("assets/music_test.wav"))
            .expect("Failed to load music");
        self.music_playback = Some(
            ctx.audio
                .play_sound_looped(music_id)
                .expect("Failed to play music"),
        );

        if let Some(pb) = self.music_playback {
            ctx.audio.set_volume(pb, 0.3);
        }

        log::info!("Audio demo ready. Space=beep, M=toggle music, Up/Down=volume");
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f64) {
        if ctx.input.is_key_just_pressed(Key::Space) {
            if let Some(id) = self.beep {
                let _ = ctx.audio.play_sound(id);
            }
        }

        if ctx.input.is_key_just_pressed(Key::KeyM) {
            if let Some(pb) = self.music_playback {
                if self.music_paused {
                    ctx.audio.resume(pb);
                } else {
                    ctx.audio.pause(pb);
                }
                self.music_paused = !self.music_paused;
            }
        }

        if ctx.input.is_key_just_pressed(Key::ArrowUp) {
            let vol = (ctx.audio.master_volume() + 0.1).min(1.0);
            ctx.audio.set_master_volume(vol);
            log::info!("Master volume: {:.0}%", vol * 100.0);
        }
        if ctx.input.is_key_just_pressed(Key::ArrowDown) {
            let vol = (ctx.audio.master_volume() - 0.1).max(0.0);
            ctx.audio.set_master_volume(vol);
            log::info!("Master volume: {:.0}%", vol * 100.0);
        }
    }

    fn draw(&mut self, _ctx: &mut GameContext) {}
}

fn main() {
    App::new()
        .with_title("Toile — Audio Demo (Week 4)")
        .with_size(640, 480)
        .run(AudioDemo {
            beep: None,
            music_playback: None,
            music_paused: false,
        });
}
