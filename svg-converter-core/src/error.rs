// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Contract C-4 — frozen error taxonomy.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    InvalidSvg(String),         // 1001
    UnsupportedFeature(String), // 1002
    ZipReadError(String),       // 1003
    ZipWriteError(String),      // 1004
    IoError(String),            // 1005
    RenderError(String),        // 1006
    Cancelled,                  // 1007
    Internal(String),           // 1099
}

impl ConversionError {
    /// Stable numeric code. Part of the frozen contract; never reused.
    pub fn code(&self) -> u16 {
        match self {
            ConversionError::InvalidSvg(_) => 1001,
            ConversionError::UnsupportedFeature(_) => 1002,
            ConversionError::ZipReadError(_) => 1003,
            ConversionError::ZipWriteError(_) => 1004,
            ConversionError::IoError(_) => 1005,
            ConversionError::RenderError(_) => 1006,
            ConversionError::Cancelled => 1007,
            ConversionError::Internal(_) => 1099,
        }
    }
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ConversionError::InvalidSvg(s) => format!("invalid SVG: {s}"),
            ConversionError::UnsupportedFeature(s) => format!("unsupported feature: {s}"),
            ConversionError::ZipReadError(s) => format!("zip read error: {s}"),
            ConversionError::ZipWriteError(s) => format!("zip write error: {s}"),
            ConversionError::IoError(s) => format!("io error: {s}"),
            ConversionError::RenderError(s) => format!("render error: {s}"),
            ConversionError::Cancelled => "cancelled".to_string(),
            ConversionError::Internal(s) => format!("internal error: {s}"),
        };
        write!(f, "[{}] {}", self.code(), msg)
    }
}

impl std::error::Error for ConversionError {}
