pub struct Time {
    pub time_scale: f32,
    pub delta: f32,
    pub unscaled_delta: f32,
    pub elapsed: f32,
    pub unscaled_elapsed: f32,
    pub tick: u64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            time_scale: 1. / 60.,
            delta: Default::default(),
            unscaled_delta: Default::default(),
            elapsed: Default::default(),
            unscaled_elapsed: Default::default(),
            tick: 60,
        }
    }
}
