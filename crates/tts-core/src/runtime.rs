
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Division {
    pub quotient: i32,
    pub remainder: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DivisionByZero;

pub fn signed_division(dividend: i32, divisor: i32) -> Result<Division, DivisionByZero> {
    if divisor == 0 {
        return Err(DivisionByZero);
    }
    Ok(Division {
        quotient: dividend.wrapping_div(divisor),
        remainder: dividend.wrapping_rem(divisor),
    })
}
