#[cfg(target_arch = "wasm32")]
#[cfg(target_arch = "wasm32")]
use dioxus_motion::prelude::{
    AnimationConfig, AnimationManager as _, AnimationMode, Duration, Spring,
};

#[cfg(target_arch = "wasm32")]
pub(crate) fn use_reveal_style(delay_ms: u64, distance_px: f32) -> String {
    let mut progress = dioxus_motion::prelude::use_motion(0.0f32);

    dioxus::prelude::use_effect(move || {
        progress.animate_to(
            1.0,
            AnimationConfig::new(AnimationMode::Spring(Spring {
                stiffness: 180.0,
                damping: 18.0,
                mass: 0.8,
                velocity: 0.0,
            }))
            .with_delay(Duration::from_millis(delay_ms))
            .with_epsilon(0.001),
        );
    });

    reveal_style(progress.get_value(), distance_px)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn use_reveal_style(_delay_ms: u64, _distance_px: f32) -> String {
    String::new()
}

#[cfg(target_arch = "wasm32")]
fn reveal_style(progress: f32, distance_px: f32) -> String {
    let opacity = progress.clamp(0.0, 1.0);
    let motion = progress.clamp(0.0, 1.08);
    let y = (1.0 - motion) * distance_px;
    let scale = 0.985 + motion * 0.015;

    format!("--motion-opacity: {opacity:.3}; --motion-y: {y:.2}px; --motion-scale: {scale:.4};")
}
