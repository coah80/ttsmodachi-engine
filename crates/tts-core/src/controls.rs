/// Caller-owned filter control values, without the C object's unrelated prefix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterControls {
    pub values: [i32; 4],
}

impl FilterControls {
    /// fn_1be21c / EU fn_1be2cc / KR fn_1be450: clamp a selected filter control.
    /// Unknown indices leave every control unchanged.
    pub fn set(&mut self, index: i32, value: i32) {
        match index {
            0..=2 => self.values[index as usize] = value.clamp(0, 127),
            3 => self.values[3] = value.clamp(0, 7997),
            _ => {}
        }
    }
}
