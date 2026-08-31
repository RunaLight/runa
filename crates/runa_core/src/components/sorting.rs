use runa_macros::Scriptable;

#[derive(Clone, Copy, Debug, Default, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct Sorting {
    pub order: i32,
    pub y_sort: bool,
    pub y_offset: f32,
}

impl Sorting {
    pub fn new(order: i32) -> Self {
        Self {
            order,
            y_sort: false,
            y_offset: 0.0,
        }
    }
}
