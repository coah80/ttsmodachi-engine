
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterControls {
    pub values: [i32; 4],
}

impl FilterControls {

    pub fn set(&mut self, index: i32, value: i32) {
        match index {
            0..=2 => self.values[index as usize] = value.clamp(0, 127),
            3 => self.values[3] = value.clamp(0, 7997),
            _ => {}
        }
    }
}
