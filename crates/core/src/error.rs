use thiserror::Error;

/// Domain errors. Every variant carries a stable machine code and an Arabic
/// message that the presentation layer may show to the user.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("invalid money format: {0}")]
    InvalidMoneyFormat(String),
    #[error("too many decimal places (max {max})")]
    TooManyDecimals { max: u8 },
    #[error("invalid number of minor units: {0}")]
    InvalidMinorUnits(u8),
    #[error("money value is out of range")]
    MoneyOverflow,
    #[error("exchange rate must be positive")]
    InvalidExchangeRate,
    #[error("invalid date, expected YYYY-MM-DD: {0}")]
    InvalidDate(String),
    #[error("a line must be either debit or credit, not both")]
    LineSideConflict,
    #[error("a line amount must be positive")]
    LineAmountNotPositive,
    #[error("base amount does not match the line side")]
    LineBaseMismatch,
    #[error("line number must be positive")]
    InvalidLineNumber,
    #[error("duplicate line number: {0}")]
    DuplicateLineNumber(u32),
    #[error("an entry needs at least two lines")]
    TooFewLines,
    #[error("entry is not balanced")]
    UnbalancedEntry,
    #[error("an entry with a zero amount cannot be posted")]
    ZeroEntry,
    #[error("entry description is required")]
    MissingDescription,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unknown value: {0}")]
    InvalidEnumValue(String),
}

impl CoreError {
    /// Stable identifier for logs and API responses.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidMoneyFormat(_) => "invalid_money_format",
            Self::TooManyDecimals { .. } => "too_many_decimals",
            Self::InvalidMinorUnits(_) => "invalid_minor_units",
            Self::MoneyOverflow => "money_overflow",
            Self::InvalidExchangeRate => "invalid_exchange_rate",
            Self::InvalidDate(_) => "invalid_date",
            Self::LineSideConflict => "line_side_conflict",
            Self::LineAmountNotPositive => "line_amount_not_positive",
            Self::LineBaseMismatch => "line_base_mismatch",
            Self::InvalidLineNumber => "invalid_line_number",
            Self::DuplicateLineNumber(_) => "duplicate_line_number",
            Self::TooFewLines => "too_few_lines",
            Self::UnbalancedEntry => "unbalanced_entry",
            Self::ZeroEntry => "zero_entry",
            Self::MissingDescription => "missing_description",
            Self::PermissionDenied(_) => "permission_denied",
            Self::InvalidEnumValue(_) => "invalid_enum_value",
        }
    }

    /// User-facing Arabic message. Technical details stay in logs only.
    pub const fn message_ar(&self) -> &'static str {
        match self {
            Self::InvalidMoneyFormat(_) => "صيغة المبلغ غير صحيحة",
            Self::TooManyDecimals { .. } => "عدد الخانات العشرية أكبر من المسموح",
            Self::InvalidMinorUnits(_) => "عدد الخانات العشرية للعملة غير صحيح",
            Self::MoneyOverflow => "المبلغ خارج النطاق المسموح",
            Self::InvalidExchangeRate => "سعر الصرف يجب أن يكون أكبر من صفر",
            Self::InvalidDate(_) => "التاريخ غير صحيح",
            Self::LineSideConflict => "كل سطر يجب أن يكون مديناً أو دائناً، لا الاثنين معاً",
            Self::LineAmountNotPositive => "قيمة السطر يجب أن تكون أكبر من صفر",
            Self::LineBaseMismatch => "المبلغ الأساسي لا يطابق اتجاه السطر",
            Self::InvalidLineNumber => "رقم السطر غير صحيح",
            Self::DuplicateLineNumber(_) => "رقم السطر مكرر",
            Self::TooFewLines => "القيد يحتاج إلى سطرين على الأقل",
            Self::UnbalancedEntry => "القيد غير متوازن: المدين لا يساوي الدائن",
            Self::ZeroEntry => "لا يمكن ترحيل قيد بمبلغ صفر",
            Self::MissingDescription => "وصف القيد مطلوب",
            Self::PermissionDenied(_) => "ليس لديك صلاحية لتنفيذ هذا الإجراء",
            Self::InvalidEnumValue(_) => "قيمة غير معروفة",
        }
    }
}
