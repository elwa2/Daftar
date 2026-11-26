//! Core accounting types and validation.
//!
//! The database enforces the same rules again (see `crates/db/migrations`).
//! This module exists so mistakes are caught before a transaction starts and
//! so the UI can explain the problem in Arabic.

use std::collections::BTreeSet;

use crate::error::CoreError;
use crate::money::{ExchangeRate, Money};

/// A calendar date in the Gregorian calendar, stored as `YYYY-MM-DD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    pub fn parse(input: &str) -> Result<Self, CoreError> {
        if !input.is_ascii() || input.len() != 10 {
            return Err(CoreError::InvalidDate(input.to_owned()));
        }
        let bytes = input.as_bytes();
        if bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(CoreError::InvalidDate(input.to_owned()));
        }

        let year = parse_digits(&input[0..4])?;
        let month = parse_digits(&input[5..7])?;
        let day = parse_digits(&input[8..10])?;

        let year = u16::try_from(year).map_err(|_| CoreError::InvalidDate(input.to_owned()))?;
        let month = u8::try_from(month).map_err(|_| CoreError::InvalidDate(input.to_owned()))?;
        let day = u8::try_from(day).map_err(|_| CoreError::InvalidDate(input.to_owned()))?;

        if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
            return Err(CoreError::InvalidDate(input.to_owned()));
        }
        if day == 0 || day > days_in_month(year, month) {
            return Err(CoreError::InvalidDate(input.to_owned()));
        }
        Ok(Self { year, month, day })
    }

    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    #[must_use]
    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.to_iso())
    }
}

fn parse_digits(input: &str) -> Result<u32, CoreError> {
    let mut value = 0_u32;
    for byte in input.bytes() {
        if !byte.is_ascii_digit() {
            return Err(CoreError::InvalidDate(input.to_owned()));
        }
        value = value * 10 + u32::from(byte - b'0');
    }
    Ok(value)
}

#[must_use]
pub const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[must_use]
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Chart-of-accounts classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl AccountType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Liability => "liability",
            Self::Equity => "equity",
            Self::Revenue => "revenue",
            Self::Expense => "expense",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "asset" => Ok(Self::Asset),
            "liability" => Ok(Self::Liability),
            "equity" => Ok(Self::Equity),
            "revenue" => Ok(Self::Revenue),
            "expense" => Ok(Self::Expense),
            other => Err(CoreError::InvalidEnumValue(other.to_owned())),
        }
    }
}

/// The side on which an account normally increases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NormalBalance {
    Debit,
    Credit,
}

impl NormalBalance {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Credit => "credit",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "debit" => Ok(Self::Debit),
            "credit" => Ok(Self::Credit),
            other => Err(CoreError::InvalidEnumValue(other.to_owned())),
        }
    }
}

/// Lifecycle of a journal entry.
///
/// `posted` is terminal. A posted entry is corrected by a posted reversal,
/// never by editing or voiding it. `void` is only reachable from `draft`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryStatus {
    Draft,
    Posted,
    Void,
}

impl EntryStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Posted => "posted",
            Self::Void => "void",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "draft" => Ok(Self::Draft),
            "posted" => Ok(Self::Posted),
            "void" => Ok(Self::Void),
            other => Err(CoreError::InvalidEnumValue(other.to_owned())),
        }
    }
}

/// One debit or credit line, in its original currency and in base currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalLine {
    pub line_no: u32,
    pub account_id: i64,
    pub description: Option<String>,
    pub currency: String,
    pub exchange_rate: ExchangeRate,
    pub debit: Money,
    pub credit: Money,
    pub base_debit: Money,
    pub base_credit: Money,
    pub cost_center_id: Option<i64>,
}

impl JournalLine {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.line_no == 0 {
            return Err(CoreError::InvalidLineNumber);
        }
        if self.account_id <= 0 {
            return Err(CoreError::InvalidEnumValue(
                "account_id must be positive".to_owned(),
            ));
        }
        if self.debit.is_negative() || self.credit.is_negative() {
            return Err(CoreError::LineAmountNotPositive);
        }

        let debit_side = self.debit.is_positive();
        let credit_side = self.credit.is_positive();
        if debit_side == credit_side {
            return Err(CoreError::LineSideConflict);
        }
        if debit_side != self.base_debit.is_positive()
            || credit_side != self.base_credit.is_positive()
        {
            return Err(CoreError::LineBaseMismatch);
        }
        Ok(())
    }
}

/// A complete journal entry before it reaches the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub entry_date: Date,
    pub description: String,
    pub currency: String,
    pub exchange_rate: ExchangeRate,
    pub lines: Vec<JournalLine>,
}

impl JournalEntry {
    /// Rules that must hold even for a draft.
    pub fn validate_draft(&self) -> Result<(), CoreError> {
        if self.description.trim().is_empty() {
            return Err(CoreError::MissingDescription);
        }
        let mut seen = BTreeSet::new();
        for line in &self.lines {
            line.validate()?;
            if !seen.insert(line.line_no) {
                return Err(CoreError::DuplicateLineNumber(line.line_no));
            }
        }
        Ok(())
    }

    /// Additional rules enforced at posting time.
    pub fn validate_for_posting(&self) -> Result<(), CoreError> {
        self.validate_draft()?;
        if self.lines.len() < 2 {
            return Err(CoreError::TooFewLines);
        }

        let mut debit = Money::ZERO;
        let mut credit = Money::ZERO;
        for line in &self.lines {
            debit = debit
                .checked_add(line.base_debit)
                .ok_or(CoreError::MoneyOverflow)?;
            credit = credit
                .checked_add(line.base_credit)
                .ok_or(CoreError::MoneyOverflow)?;
        }

        if debit.is_zero() {
            return Err(CoreError::ZeroEntry);
        }
        if debit != credit {
            return Err(CoreError::UnbalancedEntry);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(no: u32, debit: i64, credit: i64) -> JournalLine {
        JournalLine {
            line_no: no,
            account_id: 1,
            description: None,
            currency: "EGP".to_owned(),
            exchange_rate: ExchangeRate::ONE,
            debit: Money::from_minor(debit),
            credit: Money::from_minor(credit),
            base_debit: Money::from_minor(debit),
            base_credit: Money::from_minor(credit),
            cost_center_id: None,
        }
    }

    fn entry(lines: Vec<JournalLine>) -> JournalEntry {
        JournalEntry {
            entry_date: Date::parse("2026-03-01").expect("valid date"),
            description: "قيد اختبار".to_owned(),
            currency: "EGP".to_owned(),
            exchange_rate: ExchangeRate::ONE,
            lines,
        }
    }

    #[test]
    fn parses_valid_dates() {
        assert_eq!(
            Date::parse("2026-02-28").map(Date::to_iso),
            Ok("2026-02-28".to_owned())
        );
        assert_eq!(
            Date::parse("2024-02-29").map(Date::to_iso),
            Ok("2024-02-29".to_owned())
        );
        assert_eq!(
            Date::parse("2000-02-29").map(Date::to_iso),
            Ok("2000-02-29".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(Date::parse("2026-02-29").is_err());
        assert!(Date::parse("1900-02-29").is_err());
        assert!(Date::parse("2026-13-01").is_err());
        assert!(Date::parse("2026-1-1").is_err());
        assert!(Date::parse("2026-01-32").is_err());
        assert!(Date::parse("٢٠٢٦-٠١-٠١").is_err());
    }

    #[test]
    fn accepts_balanced_entry() {
        let value = entry(vec![line(1, 10_000, 0), line(2, 0, 10_000)]);
        assert_eq!(value.validate_for_posting(), Ok(()));
    }

    #[test]
    fn rejects_unbalanced_entry() {
        let value = entry(vec![line(1, 10_000, 0), line(2, 0, 9_000)]);
        assert_eq!(
            value.validate_for_posting(),
            Err(CoreError::UnbalancedEntry)
        );
    }

    #[test]
    fn rejects_both_sides_on_one_line() {
        let value = entry(vec![line(1, 100, 100), line(2, 0, 0)]);
        assert_eq!(value.validate_draft(), Err(CoreError::LineSideConflict));
    }

    #[test]
    fn rejects_single_line_posting() {
        let value = entry(vec![line(1, 100, 0)]);
        assert_eq!(value.validate_for_posting(), Err(CoreError::TooFewLines));
    }

    #[test]
    fn rejects_duplicate_line_numbers() {
        let value = entry(vec![line(1, 100, 0), line(1, 0, 100)]);
        assert_eq!(
            value.validate_draft(),
            Err(CoreError::DuplicateLineNumber(1))
        );
    }
}
