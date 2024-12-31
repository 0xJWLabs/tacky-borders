use std::time::Instant;

use super::wrapper::AnimationEngineVec;
use super::AnimationsConfig;
use crate::core::timer::CustomTimer;
use crate::error::LogIfErr;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationProgress {
    pub fade: f32,
    pub spiral: f32,
    pub angle: f32,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationFlags {
    pub fade_to_visible: bool,
    pub should_fade: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimationManager {
    active: AnimationEngineVec,
    inactive: AnimationEngineVec,
    fps: i32,
    timer: Option<CustomTimer>,
    last_animation_time: Option<Instant>,
    pub progress: AnimationProgress,
    pub flags: AnimationFlags,
}

impl AnimationManager {
    pub const fn fps(&self) -> f32 {
        self.fps as f32
    }
    pub fn get_active_animation(&self) -> &AnimationEngineVec {
        &self.active
    }

    pub fn get_inactive_animation(&self) -> &AnimationEngineVec {
        &self.inactive
    }

    pub fn has_active_or_inactive_animations(&self) -> bool {
        !self.active.is_empty() || !self.inactive.is_empty()
    }

    pub fn set_timer(&mut self, hwnd: isize) -> anyhow::Result<()> {
        if self.timer.is_none() && self.has_active_or_inactive_animations() {
            let timer_duration = (1000.0 / self.fps()) as u64;
            let timer = CustomTimer::start(hwnd, timer_duration)?;
            self.timer = Some(timer);
            self.last_animation_time = Some(Instant::now());
        }

        Ok(())
    }

    pub fn kill_timer(&mut self, hwnd: isize) -> anyhow::Result<()> {
        if self.timer.is_some() && self.has_active_or_inactive_animations() {
            CustomTimer::stop(hwnd).log_if_err();
            self.timer = None;
        }

        Ok(())
    }

    pub fn last_animation_time(&self) -> Instant {
        self.last_animation_time.unwrap_or(Instant::now())
    }

    pub fn set_last_animation_time(&mut self, time: Option<Instant>) {
        let time = time.unwrap_or(Instant::now());
        self.last_animation_time = Some(time);
    }
}

impl TryFrom<AnimationsConfig> for AnimationManager {
    type Error = anyhow::Error;
    fn try_from(value: AnimationsConfig) -> Result<AnimationManager, Self::Error> {
        let active = AnimationEngineVec::try_from(value.active.clone().unwrap_or_default())?;
        let inactive = AnimationEngineVec::try_from(value.inactive.clone().unwrap_or_default())?;
        Ok(AnimationManager {
            active,
            inactive,
            fps: value.fps.unwrap_or(60),
            ..Default::default()
        })
    }
}
