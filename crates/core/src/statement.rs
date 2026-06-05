use chrono::NaiveDate;

use crate::models::{Balance, StatementNumber, Transaction};

/// A complete MT940 statement
///
/// Each statement corresponds to one `:20:` block in the MT940 file.
/// A single MT940 file may contain multiple statements (each starting
/// with a `:20:` tag).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// :20: Transaction Reference Number (mandatory, 16c max)
    pub transaction_reference: String,

    /// :21: Related Reference (optional, 16c max)
    pub related_reference: Option<String>,

    /// :25: Account Identification (mandatory, 35x max)
    pub account_identification: String,

    /// :28C: Statement Number / Sequence Number (mandatory)
    pub statement_number: StatementNumber,

    /// :60F: or :60M: Opening Balance
    pub opening_balance: Balance,

    /// :62F: or :62M: Closing Balance
    pub closing_balance: Balance,

    /// :64: Closing Available Balance (optional)
    pub closing_available: Option<Balance>,

    /// :65: Forward Available Balance (optional)
    pub forward_available: Option<Balance>,

    /// :61: + :86: Transaction entries (optionally repeated)
    pub transactions: Vec<Transaction>,

    /// Standalone :86: Information to Account Owner (no preceding :61:)
    pub info_to_owner: Option<String>,

    /// Whether the :60F:/:60M: opening balance tag was parsed.
    /// Used for validation at statement-finalize time.
    pub has_opening_balance: bool,

    /// Whether the :62F:/:62M: closing balance tag was parsed.
    pub has_closing_balance: bool,
}

impl Statement {
    /// Create a new statement with the given transaction reference.
    /// The caller must populate mandatory tags before finalizing.
    pub fn new(transaction_reference: String) -> Self {
        Statement {
            transaction_reference,
            related_reference: None,
            account_identification: String::new(),
            statement_number: StatementNumber {
                statement: String::new(),
                sequence: None,
            },
            opening_balance: Balance::default(),
            closing_balance: Balance::default(),
            closing_available: None,
            forward_available: None,
            transactions: Vec::new(),
            info_to_owner: None,
            has_opening_balance: false,
            has_closing_balance: false,
        }
    }

    /// Set the opening balance and mark the tag as parsed.
    pub(crate) fn set_opening_balance(&mut self, bal: Balance) {
        self.opening_balance = bal;
        self.has_opening_balance = true;
    }

    /// Set the closing balance and mark the tag as parsed.
    pub(crate) fn set_closing_balance(&mut self, bal: Balance) {
        self.closing_balance = bal;
        self.has_closing_balance = true;
    }
}

impl Statement {
    /// Returns the statement currency (inherited from opening balance).
    pub fn currency(&self) -> &str {
        &self.opening_balance.currency
    }

    /// Returns the statement date (from the closing balance).
    pub fn statement_date(&self) -> NaiveDate {
        self.closing_balance.date
    }
}
