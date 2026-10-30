/// The two register results returned by the RVCT signed division library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Division {
    pub quotient: i32,
    pub remainder: i32,
}

/// Division by zero enters the application's arithmetic-error handler on ARM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DivisionByZero;

/// fn_30209c / EU fn_302258 / KR fn_302304: signed quotient and remainder.
/// Division truncates toward zero; MIN / -1 returns MIN and zero.
/// Zero divisors return an error for the caller to handle instead of invoking a global hook.
pub fn signed_division(dividend: i32, divisor: i32) -> Result<Division, DivisionByZero> {
    if divisor == 0 {
        return Err(DivisionByZero);
    }
    Ok(Division {
        quotient: dividend.wrapping_div(divisor),
        remainder: dividend.wrapping_rem(divisor),
    })
}
