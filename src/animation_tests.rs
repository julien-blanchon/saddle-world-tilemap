use super::*;

#[test]
fn animation_frame_progression_wraps_deterministically() {
    let animation = TileAnimation {
        frames: vec![
            TileAnimationFrame {
                atlas_index: 3,
                duration_seconds: 0.25,
            },
            TileAnimationFrame {
                atlas_index: 4,
                duration_seconds: 0.5,
            },
            TileAnimationFrame {
                atlas_index: 5,
                duration_seconds: 0.25,
            },
        ],
    };

    assert_eq!(animation.frame_index_at(0.0), 0);
    assert_eq!(animation.frame_index_at(0.24), 0);
    assert_eq!(animation.frame_index_at(0.25), 1);
    assert_eq!(animation.frame_index_at(0.74), 1);
    assert_eq!(animation.frame_index_at(0.75), 2);
    assert_eq!(animation.frame_index_at(1.0), 0);
}

#[test]
fn zero_duration_frames_fall_back_to_epsilon_without_panicking() {
    let animation = TileAnimation {
        frames: vec![
            TileAnimationFrame {
                atlas_index: 7,
                duration_seconds: 0.0,
            },
            TileAnimationFrame {
                atlas_index: 8,
                duration_seconds: 0.0,
            },
        ],
    };

    assert_eq!(animation.atlas_index_at(0.0), 7);
    assert_eq!(animation.frame_index_at(f32::EPSILON * 1.5), 1);
}
