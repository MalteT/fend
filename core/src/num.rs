use crate::err::{IntErr, Interrupt, Never};
use std::{cell::Cell, fmt};

mod base;
mod bigrat;
mod biguint;
mod complex;
mod formatting_style;
mod real;
mod unit;

pub use formatting_style::FormattingStyle;

pub type Number = unit::UnitValue;
pub type Base = base::Base;
pub type BaseOutOfRangeError = base::BaseOutOfRangeError;
pub type InvalidBasePrefixError = base::InvalidBasePrefixError;

// Small formatter helper
pub fn to_string<
    I: Interrupt,
    R,
    F: Fn(&mut fmt::Formatter) -> Result<R, IntErr<fmt::Error, I>>,
>(
    func: F,
) -> Result<(String, R), IntErr<Never, I>> {
    struct Fmt<I: Interrupt, R, F: Fn(&mut fmt::Formatter) -> Result<R, IntErr<fmt::Error, I>>> {
        format: F,
        error: Cell<Option<IntErr<Never, I>>>,
        result: Cell<Option<R>>,
    }

    impl<F, R, I: Interrupt> fmt::Display for Fmt<I, R, F>
    where
        F: Fn(&mut fmt::Formatter) -> Result<R, IntErr<fmt::Error, I>>,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            let interrupt = match (self.format)(f) {
                Ok(res) => {
                    self.result.set(Some(res));
                    return Ok(());
                }
                Err(IntErr::Interrupt(i)) => i,
                Err(IntErr::Error(e)) => return Err(e),
            };
            self.error.set(Some(IntErr::Interrupt(interrupt)));
            Ok(())
        }
    }

    let fmt = Fmt {
        format: func,
        error: Cell::new(None),
        result: Cell::new(None),
    };
    let string = fmt.to_string();
    if let Some(e) = fmt.error.into_inner() {
        return Err(e);
    }
    Ok((string, fmt.result.into_inner().unwrap()))
}

pub struct ValueTooLarge<T: fmt::Display> {
    max_allowed: T,
}

impl<T: fmt::Display> fmt::Display for ValueTooLarge<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Value must be less than or equal to {}",
            self.max_allowed
        )?;
        Ok(())
    }
}
impl<T: fmt::Display> crate::err::Error for ValueTooLarge<T> {}

#[derive(Debug)]
pub enum IntegerPowerError {
    ExponentTooLarge,
    ZeroToThePowerOfZero,
}

impl fmt::Display for IntegerPowerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::ExponentTooLarge => write!(f, "Exponent too large"),
            Self::ZeroToThePowerOfZero => write!(f, "Zero to the power of zero is undefined"),
        }
    }
}
impl crate::err::Error for IntegerPowerError {}

#[derive(Debug)]
pub struct DivideByZero {}
impl fmt::Display for DivideByZero {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Division by zero")
    }
}
impl crate::err::Error for DivideByZero {}
