use runa_macros::Scriptable;

#[derive(Clone, Debug, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct SpriteSheet {
    pub columns: u32,
    pub rows: u32,
}

impl SpriteSheet {
    pub fn new(columns: u32, rows: u32) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
        }
    }

    pub fn frame_count(&self) -> u32 {
        self.columns.saturating_mul(self.rows).max(1)
    }

    pub fn uv_rect_for_frame(&self, frame: u32) -> [f32; 4] {
        let columns = self.columns.max(1);
        let rows = self.rows.max(1);
        let frame = frame.min(self.frame_count().saturating_sub(1));
        let col = frame % columns;
        let row = frame / columns;
        let width = 1.0 / columns as f32;
        let height = 1.0 / rows as f32;

        [col as f32 * width, row as f32 * height, width, height]
    }
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

#[derive(Clone, Debug, Scriptable)]
#[script(crate = "::runa_script_api", not_addable, builtin)]
pub struct SpriteAnimationClip {
    pub name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub fps: f32,
    pub looping: bool,
}

impl SpriteAnimationClip {
    pub fn new(name: impl Into<String>, start_frame: u32, end_frame: u32, fps: f32) -> Self {
        Self {
            name: name.into(),
            start_frame,
            end_frame: end_frame.max(start_frame),
            fps,
            looping: true,
        }
    }
}

#[derive(Clone, Debug, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct SpriteAnimator {
    pub sheet: SpriteSheet,
    pub clips: Vec<SpriteAnimationClip>,
    pub current_clip: Option<String>,
    pub current_frame: u32,
    pub playing: bool,
    #[script(skip)]
    accumulator: f32,
}

impl SpriteAnimator {
    pub fn new(sheet: SpriteSheet) -> Self {
        Self {
            sheet,
            clips: vec![SpriteAnimationClip::new("Default", 0, 0, 12.0)],
            current_clip: Some("Default".to_string()),
            current_frame: 0,
            playing: true,
            accumulator: 0.0,
        }
    }

    pub fn with_clip(mut self, clip: SpriteAnimationClip) -> Self {
        if self.clips.len() == 1 && self.clips[0].name == "Default" {
            self.clips.clear();
        }
        if self.current_clip.is_none() {
            self.current_clip = Some(clip.name.clone());
            self.current_frame = clip.start_frame;
        }
        self.clips.push(clip);
        self
    }

    pub fn from_clips(
        sheet: SpriteSheet,
        clips: Vec<SpriteAnimationClip>,
        current_clip: Option<String>,
        current_frame: u32,
        playing: bool,
    ) -> Self {
        let mut animator = Self {
            sheet,
            clips,
            current_clip,
            current_frame,
            playing,
            accumulator: 0.0,
        };
        animator.clamp_state();
        animator
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_frame = self.active_clip().map(|clip| clip.start_frame).unwrap_or(0);
        self.accumulator = 0.0;
    }

    pub fn play_clip(&mut self, name: &str) -> bool {
        let Some(clip) = self.clips.iter().find(|clip| clip.name == name) else {
            return false;
        };
        if self.current_clip.as_deref() != Some(name) {
            self.current_clip = Some(clip.name.clone());
            self.current_frame = clip.start_frame;
            self.accumulator = 0.0;
        }
        self.playing = true;
        true
    }

    pub fn set_sheet(&mut self, columns: u32, rows: u32) {
        self.sheet = SpriteSheet::new(columns, rows);
        self.current_frame = self
            .current_frame
            .min(self.sheet.frame_count().saturating_sub(1));
    }

    pub fn tick(&mut self, dt: f32) -> [f32; 4] {
        self.clamp_state();
        let Some(clip) = self.active_clip().cloned() else {
            return self.sheet.uv_rect_for_frame(self.current_frame);
        };

        if self.playing && clip.fps > f32::EPSILON {
            self.accumulator += dt.max(0.0);
            let frame_time = 1.0 / clip.fps;
            while self.accumulator >= frame_time {
                self.accumulator -= frame_time;
                self.advance_frame(&clip);
            }
        }

        self.sheet.uv_rect_for_frame(self.current_frame)
    }

    pub fn active_clip(&self) -> Option<&SpriteAnimationClip> {
        let name = self.current_clip.as_deref()?;
        self.clips.iter().find(|clip| clip.name == name)
    }

    fn advance_frame(&mut self, clip: &SpriteAnimationClip) {
        if self.current_frame < clip.end_frame {
            self.current_frame += 1;
            return;
        }

        if clip.looping {
            self.current_frame = clip.start_frame;
        } else {
            self.current_frame = clip.end_frame;
            self.playing = false;
        }
    }

    fn clamp_state(&mut self) {
        let max_frame = self.sheet.frame_count().saturating_sub(1);
        for clip in &mut self.clips {
            clip.start_frame = clip.start_frame.min(max_frame);
            clip.end_frame = clip.end_frame.max(clip.start_frame).min(max_frame);
            if clip.name.trim().is_empty() {
                clip.name = "Clip".to_string();
            }
        }

        if self.clips.is_empty() {
            self.clips
                .push(SpriteAnimationClip::new("Default", 0, max_frame, 12.0));
        }

        if self
            .current_clip
            .as_ref()
            .and_then(|name| self.clips.iter().find(|clip| &clip.name == name))
            .is_none()
        {
            self.current_clip = self.clips.first().map(|clip| clip.name.clone());
        }

        if let Some(clip) = self.active_clip() {
            self.current_frame = self.current_frame.clamp(clip.start_frame, clip.end_frame);
        } else {
            self.current_frame = self.current_frame.min(max_frame);
        }
    }
}

impl Default for SpriteAnimator {
    fn default() -> Self {
        Self::new(SpriteSheet::default())
    }
}
