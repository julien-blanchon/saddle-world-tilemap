use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileAnimationFrame {
    pub atlas_index: u32,
    pub duration_seconds: f32,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileAnimation {
    pub frames: Vec<TileAnimationFrame>,
}

impl TileAnimation {
    #[must_use]
    pub fn uniform(indices: impl IntoIterator<Item = u32>, duration_seconds: f32) -> Self {
        Self {
            frames: indices
                .into_iter()
                .map(|atlas_index| TileAnimationFrame {
                    atlas_index,
                    duration_seconds,
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn total_duration(&self) -> f32 {
        self.frames
            .iter()
            .map(|frame| frame.duration_seconds.max(f32::EPSILON))
            .sum()
    }

    #[must_use]
    pub fn atlas_index_at(&self, elapsed_seconds: f32) -> u32 {
        if self.frames.is_empty() {
            return 0;
        }

        let total_duration = self.total_duration();
        let mut cursor = elapsed_seconds.rem_euclid(total_duration);

        for frame in &self.frames {
            let duration = frame.duration_seconds.max(f32::EPSILON);
            if cursor < duration {
                return frame.atlas_index;
            }
            cursor -= duration;
        }

        self.frames.last().map_or(0, |frame| frame.atlas_index)
    }

    #[must_use]
    pub fn frame_index_at(&self, elapsed_seconds: f32) -> usize {
        if self.frames.is_empty() {
            return 0;
        }

        let total_duration = self.total_duration();
        let mut cursor = elapsed_seconds.rem_euclid(total_duration);

        for (index, frame) in self.frames.iter().enumerate() {
            let duration = frame.duration_seconds.max(f32::EPSILON);
            if cursor < duration {
                return index;
            }
            cursor -= duration;
        }

        self.frames.len() - 1
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TileAnimationRuntimeState {
    pub elapsed_seconds: f32,
    pub frame_index: usize,
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
