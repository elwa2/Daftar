//! Integer money and exchange-rate arithmetic.
//!
//! Amounts are signed 64-bit counts of the currency's smallest unit. Floating
//! point is never used. Exchange rates are scaled by [`RATE_SCALE`].
//!
//! Conversion rounds **half away from zero** and is applied per line. The
//! posting service must therefore add an explicit rounding line when the
//! converted lines do not balance.

use crate::error::CoreError;

/// Fixed scale for exchange rates: `1_000_000` represents a rate of `1.0`.
pub const RATE_SCALE: i64 = 1_000_000;

/// An amount in the smallest unit of a currency (piastres, cents, fils...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn from_minor(minor: i64) -> Self {
        Self(minor)
    }

    #[must_use]
    pub const fn minor(self) -> i64 {
        self.0
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    #[must_use]
    pub fn checked_neg(self) -> Option<Self> {
        self.0.checked_neg().map(Self)
    }

    /// Parses a plain decimal string such as `1234.56`.
    ///
    /// Thousands separators, spaces inside the number and scientific notation
    /// are rejected on purpose: the UI layer formats values before display and
    /// sends plain decimals to the engine.
    pub fn parse(input: &str, minor_units: u8) -> Result<Self, CoreError> {
        if minor_units > 6 {
            return Err(CoreError::InvalidMinorUnits(minor_units));
        }
        let text = input.trim();
        if text.is_empty() {
            return Err(CoreError::InvalidMoneyFormat(input.to_owned()));
        }

        let (negative, unsigned) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        if unsigned.is_empty() {
            return Err(CoreError::InvalidMoneyFormat(input.to_owned()));
        }

        let mut parts = unsigned.split('.');
        let whole = parts.next().unwrap_or_default();
        let fraction = parts.next();
        if parts.next().is_some() {
            return Err(CoreError::InvalidMoneyFormat(input.to_owned()));
        }
        if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CoreError::InvalidMoneyFormat(input.to_owned()));
        }

        let fraction_value = match fraction {
            Some(value) => {
                if value.is_empty()
                    || value.len() > usize::from(minor_units)
                    || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    if !value.is_empty() && value.len() > usize::from(minor_units) {
                        return Err(CoreError::TooManyDecimals { max: minor_units });
                    }
                    return Err(CoreError::InvalidMoneyFormat(input.to_owned()));
                }
                let mut padded = String::with_capacity(usize::from(minor_units));
                padded.push_str(value);
                while padded.len() < usize::from(minor_units) {
                    padded.push('0');
                }
                padded
                    .parse::<i64>()
                    .map_err(|_| CoreError::MoneyOverflow)?
            }
            None => 0,
        };

        let scale = 10_i64
            .checked_pow(u32::from(minor_units))
            .ok_or(CoreError::MoneyOverflow)?;
        let whole_value = whole.parse::<i64>().map_err(|_| CoreError::MoneyOverflow)?;
        let minor = whole_value
            .checked_mul(scale)
            .and_then(|value| value.checked_add(fraction_value))
            .ok_or(CoreError::MoneyOverflow)?;

        if negative {
            minor
                .checked_neg()
                .map(Self)
                .ok_or(CoreError::MoneyOverflow)
        } else {
            Ok(Self(minor))
        }
    }

    /// Formats the amount with exactly `minor_units` decimal places.
    #[must_use]
    pub fn format(self, minor_units: u8) -> String {
        if minor_units == 0 {
            return self.0.to_string();
        }
        let units = u32::from(minor_units.min(6));
        let factor = 10_u64.pow(units);
        let negative = self.0 < 0;
        let absolute = self.0.unsigned_abs();
        let whole = absolute / factor;
        let fraction = absolute % factor;
        let sign = if negative { "-" } else { "" };
        format!("{sign}{whole}.{fraction:0width$}", width = units as usize)
    }
}

/// A positive exchange rate scaled by [`RATE_SCALE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExchangeRate(i64);

impl ExchangeRate {
    pub const ONE: Self = Self(RATE_SCALE);

    pub fn from_scaled(scaled: i64) -> Result<Self, CoreError> {
        if scaled > 0 {
            Ok(Self(scaled))
        } else {
            Err(CoreError::InvalidExchangeRate)
        }
    }

    #[must_use]
    pub const fn scaled(self) -> i64 {
        self.0
    }

    /// Converts `amount` from a currency with `from_minor_units` to a currency
    /// with `to_minor_units`, rounding half away from zero.
    pub fn convert(
        self,
        amount: Money,
        from_minor_units: u8,
        to_minor_units: u8,
    ) -> Result<Money, CoreError> {
        if from_minor_units > 6 || to_minor_units > 6 {
            return Err(CoreError::InvalidMinorUnits(
                from_minor_units.max(to_minor_units),
            ));
        }
        let numerator =
            i128::from(amount.minor()) * i128::from(self.0) * pow10_i128(to_minor_units);
        let denominator = i128::from(RATE_SCALE) * pow10_i128(from_minor_units);

        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        let rounded = if remainder.abs() * 2 >= denominator {
            quotient + numerator.signum()
        } else {
            quotient
        };

        i64::try_from(rounded)
            .map(Money)
            .map_err(|_| CoreError::MoneyOverflow)
    }
}

fn pow10_i128(exponent: u8) -> i128 {
    let mut value = 1_i128;
    let mut remaining = exponent;
    while remaining > 0 {
        value *= 10;
        remaining -= 1;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_decimals() {
        assert_eq!(Money::parse("1234.56", 2), Ok(Money::from_minor(123_456)));
        assert_eq!(Money::parse("-0.05", 2), Ok(Money::from_minor(-5)));
        assert_eq!(Money::parse("+7", 0), Ok(Money::from_minor(7)));
        assert_eq!(Money::parse("0.5", 2), Ok(Money::from_minor(50)));
        assert_eq!(Money::parse("0.005", 3), Ok(Money::from_minor(5)));
    }

    #[test]
    fn rejects_malformed_decimals() {
        assert!(Money::parse("", 2).is_err());
        assert!(Money::parse("1,000.00", 2).is_err());
        assert!(Money::parse("1.2.3", 2).is_err());
        assert!(Money::parse("1.", 2).is_err());
        assert!(Money::parse("1.234", 2).is_err());
        assert!(Money::parse("-", 2).is_err());
        assert!(Money::parse("abc", 2).is_err());
    }

    #[test]
    fn formats_with_fixed_scale() {
        assert_eq!(Money::from_minor(123_456).format(2), "1234.56");
        assert_eq!(Money::from_minor(-5).format(2), "-0.05");
        assert_eq!(Money::from_minor(7).format(0), "7");
        assert_eq!(Money::from_minor(1_000).format(3), "1.000");
    }

    #[test]
    fn converts_between_currencies() {
        let rate = ExchangeRate::from_scaled(1_500_000).expect("valid rate");
        let egp = Money::from_minor(10_000);
        assert_eq!(rate.convert(egp, 2, 2), Ok(Money::from_minor(15_000)));
    }

    #[test]
    fn conversion_rounds_half_away_from_zero() {
        let rate = ExchangeRate::from_scaled(333_333).expect("valid rate");
        // 0.05 * 0.333333 = 0.01666665 -> 0.02
        assert_eq!(
            rate.convert(Money::from_minor(5), 2, 2),
            Ok(Money::from_minor(2))
        );
        // -0.05 * 0.333333 = -0.01666665 -> -0.02
        assert_eq!(
            rate.convert(Money::from_minor(-5), 2, 2),
            Ok(Money::from_minor(-2))
        );
    }

    #[test]
    fn conversion_respects_minor_units() {
        let rate = ExchangeRate::from_scaled(1_000 * RATE_SCALE).expect("valid rate");
        assert_eq!(
            rate.convert(Money::from_minor(1_000), 3, 2),
            Ok(Money::from_minor(100_000))
        );
    }

    #[test]
    fn rejects_non_positive_rates() {
        assert!(ExchangeRate::from_scaled(0).is_err());
        assert!(ExchangeRate::from_scaled(-1).is_err());
    }
}
