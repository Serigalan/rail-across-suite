// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! `PayloadError`: a payload this build will not take.

use serde_json::{Value, json};

/// A payload this build will not take: `code` the key the interface
/// translates (`invalid_payload`, `unsupported_version`, `stages_unsupported`,
/// …) and `details` what was found, as "path: what" lines.
#[derive(Debug, Clone, PartialEq)]
pub struct PayloadError {
    pub code: String,
    pub details: Vec<String>,
}

impl PayloadError {
    pub fn new(code: &str) -> Self {
        PayloadError {
            code: code.to_string(),
            details: Vec::new(),
        }
    }

    pub fn with(code: &str, details: Vec<String>) -> Self {
        PayloadError {
            code: code.to_string(),
            details,
        }
    }

    /// As the core's face hands it to the JavaScript `PayloadError`.
    pub fn to_json(&self) -> Value {
        json!({ "code": self.code, "details": self.details })
    }
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.details.is_empty() {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}: {}", self.code, self.details.iter().take(5).cloned().collect::<Vec<_>>().join("; "))
        }
    }
}

impl std::error::Error for PayloadError {}

pub type Result<T> = std::result::Result<T, PayloadError>;
