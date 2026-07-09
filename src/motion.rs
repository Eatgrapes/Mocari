//! Cubism motion playback.
//!
//! Load a motion with [`crate::motion::load_motion`], wrap it in a
//! [`MotionPlayer`], and call [`MotionPlayer::tick`] plus [`MotionPlayer::apply`]
//! each frame before updating runtime meshes.

use std::{fs, path::Path};

use crate::{
    json::{
        Motion3, apply_motion_fade, motion_fade_in_weight, motion_fade_out_weight,
        parameter_curve_fade_weight,
    },
    runtime::ModelRuntime,
};

const PARAMETER_TARGET: &str = "Parameter";
const PART_OPACITY_TARGET: &str = "PartOpacity";

#[derive(Debug, Clone)]
/// Plays a parsed `motion3.json` animation against a [`ModelRuntime`].
///
/// A player owns playback time and blend weight. It does not update meshes by
/// itself; call [`ModelRuntime::update_meshes`] after applying one or more
/// players.
pub struct MotionPlayer {
    motion: Motion3,
    time: f32,
    weight: f32,
    finished: bool,
}

impl MotionPlayer {
    /// Creates a player at time `0.0` with full weight.
    pub fn new(motion: Motion3) -> Self {
        Self {
            motion,
            time: 0.0,
            weight: 1.0,
            finished: false,
        }
    }

    /// Returns the motion data owned by this player.
    pub fn motion(&self) -> &Motion3 {
        &self.motion
    }

    /// Returns the current playback time in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Returns the player's global blend weight.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets the player's global blend weight, clamped to `0.0..=1.0`.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight.clamp(0.0, 1.0);
    }

    /// Returns whether a non-looping motion has reached its end.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Restarts playback from the beginning.
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.finished = false;
    }

    /// Advances playback time by `delta_seconds`.
    ///
    /// Negative deltas are treated as zero. Looping motions wrap at their
    /// declared duration; non-looping motions stop at the end.
    pub fn tick(&mut self, delta_seconds: f32) {
        if self.finished {
            return;
        }

        self.time += delta_seconds.max(0.0);
        let duration = self.motion.meta().duration();
        if duration <= 0.0 {
            return;
        }

        if self.motion.meta().is_looping() {
            self.time %= duration;
        } else if self.time >= duration {
            self.time = duration;
            self.finished = true;
        }
    }

    /// Applies the current motion sample to a model runtime.
    ///
    /// Curves targeting unknown parameters or parts are ignored. Call
    /// [`ModelRuntime::update_meshes`] after all motion and expression players
    /// have been applied for the frame.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let duration = self.motion.meta().duration();
        let end_time = if self.motion.meta().is_looping() {
            -1.0
        } else {
            duration
        };
        let fade_in = motion_fade_in_weight(self.time, 0.0, 0.0);
        let fade_out = motion_fade_out_weight(self.time, end_time, 0.0);

        for curve in self.motion.curves() {
            let Some(sampled) = curve.sample(self.time) else {
                continue;
            };
            let curve_weight = parameter_curve_fade_weight(
                self.weight,
                fade_in,
                fade_out,
                curve.fade_in_time(),
                curve.fade_out_time(),
                self.time,
                0.0,
                end_time,
            );

            match curve.target() {
                PARAMETER_TARGET => {
                    let Some(index) = runtime.parameter_index(curve.id()) else {
                        continue;
                    };
                    let Some(current) = runtime.parameter_value_by_index(index) else {
                        continue;
                    };
                    let value = apply_motion_fade(current, sampled, curve_weight);
                    runtime.set_parameter_by_index(index, value);
                }
                PART_OPACITY_TARGET => {
                    let Some(index) = runtime.part_index(curve.id()) else {
                        continue;
                    };
                    let value = apply_motion_fade(1.0, sampled, curve_weight);
                    runtime.set_part_opacity_by_index(index, value);
                }
                _ => {}
            }
        }
    }
}

/// Loads a Cubism `motion3.json` file from disk.
pub fn load_motion(path: impl AsRef<Path>) -> Result<Motion3, MotionLoadError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| MotionLoadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Motion3::from_json_str(&source).map_err(MotionLoadError::Parse)
}

#[derive(Debug, thiserror::Error)]
/// Errors that can occur while loading a motion file.
pub enum MotionLoadError {
    /// The motion file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path of the file that failed to load.
        path: String,
        /// Original I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The motion JSON was invalid or unsupported.
    #[error("failed to parse motion3: {0}")]
    Parse(#[source] crate::Error),
}
