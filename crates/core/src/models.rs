use std::collections::HashMap;
use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Debit or credit indicator for transactions and balances
///
/// In camt.053 output, this maps to `<CdtDbtInd>` (DBIT or CRDT).
/// Reversal indicators (RD, RC) are captured as separate debit/credit
/// type with the reversal flag set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebitOrCredit {
    /// Regular debit: money leaving the account
    Debit,
    /// Regular credit: money entering the account
    Credit,
    /// Reversal of a previous debit entry: treated as credit
    ReversalDebit,
    /// Reversal of a previous credit entry: treated as debit
    ReversalCredit,
}

impl DebitOrCredit {
    /// Returns true if this entry represents money entering the account
    /// (Credit or ReversalDebit).
    pub fn is_credit(&self) -> bool {
        matches!(self, Self::Credit | Self::ReversalDebit)
    }

    /// Returns true if this entry represents money leaving the account
    /// (Debit or ReversalCredit).
    pub fn is_debit(&self) -> bool {
        matches!(self, Self::Debit | Self::ReversalCredit)
    }

    /// Returns true if this is a reversal.
    pub fn is_reversal(&self) -> bool {
        matches!(self, Self::ReversalDebit | Self::ReversalCredit)
    }

    /// Returns the effective (non-reversal) debit/credit classification.
    pub fn effective(&self) -> Self {
        match self {
            Self::ReversalDebit => Self::Credit,
            Self::ReversalCredit => Self::Debit,
            other => *other,
        }
    }
}

impl fmt::Display for DebitOrCredit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Debit => write!(f, "D"),
            Self::Credit => write!(f, "C"),
            Self::ReversalDebit => write!(f, "RD"),
            Self::ReversalCredit => write!(f, "RC"),
        }
    }
}

/// A balance entry: opening, closing, or available balance
///
/// Balances always store a positive (`amount`) with the sign disambiguated
/// by the `debit_credit` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    /// Whether this is an intermediate (M) or final (F) balance
    pub is_intermediate: bool,
    /// Debit or credit indicator for the balance
    pub debit_credit: DebitOrCredit,
    /// Balance date (from the 6-digit YYMMDD field)
    pub date: NaiveDate,
    /// ISO 4217 currency code (3 characters)
    pub currency: String,
    /// Balance amount: always positive (sign in `debit_credit`)
    pub amount: Decimal,
}

/// Parsed representation of the `:28C:` Statement Number / Sequence Number tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementNumber {
    /// Statement number (first part before `/`)
    pub statement: String,
    /// Sequence number (second part after `/`, if present)
    pub sequence: Option<String>,
}

/// A single transaction entry: parsed from a :61: / :86: tag pair
///
/// The `amount` field is always stored as a **positive** value.
/// The sign is indicated by the `debit_credit` field:
///   - `Debit` or `ReversalCredit` → money leaving the account
///   - `Credit` or `ReversalDebit` → money entering the account
///
/// Use [`Transaction::signed_amount`] to get amounts with sign applied
/// for display or export (e.g., −1500.00 for debits).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Value date (YYMMDD from :61: positions 0-5)
    pub value_date: NaiveDate,
    /// Entry date (MMDD from :61: positions 6-9), inferred from value_date
    pub entry_date: Option<NaiveDate>,
    /// Debit or credit indicator
    pub debit_credit: DebitOrCredit,
    /// Optional funds code (1 character; third currency letter variant)
    pub funds_code: Option<String>,
    /// Transaction amount: always positive (sign in `debit_credit`)
    pub amount: Decimal,
    /// Transaction type ID code (e.g., "NTRF", "NMSC", "N166")
    pub transaction_type: String,
    /// Customer reference from :61:
    pub customer_reference: String,
    /// Bank reference from :61: (the `//bankref` portion)
    pub bank_reference: Option<String>,
    /// Supplementary details from :61: continuation lines
    pub supplementary: Option<String>,
    /// Raw, concatenated :86: text: always preserved
    pub details: String,
    /// Parsed key-value pairs from :86: text when a dialect decoder matches.
    /// `None` means unstructured: the raw `details` text is the fallback
    pub structured_details: Option<HashMap<String, String>>,
}

impl Transaction {
    /// Returns the amount with sign applied.
    ///
    /// Negative for debits (D, RC), positive for credits (C, RD).
    /// ```text
    /// Debit (D) → -1500.00
    /// Credit (C) →  3250.75
    /// ```
    pub fn signed_amount(&self) -> Decimal {
        if self.debit_credit.is_debit() {
            -self.amount
        } else {
            self.amount
        }
    }

    /// Returns the counterparty name from structured details.
    ///
    /// Resolution order:
    ///   1. SWIFT structured: `"NAME"` key
    ///   2. German GVC: `"32"` key + `"33"` key (space-joined)
    ///   3. Angular: `"27"` key
    ///   4. Falls back to empty string
    pub fn counterparty(&self) -> Option<&str> {
        self.structured_details.as_ref().and_then(|sd| {
            sd.get("NAME").or_else(|| sd.get("32")).or_else(|| sd.get("27")).map(|s| s.as_str())
        })
    }
}
