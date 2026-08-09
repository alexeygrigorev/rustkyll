//! Build-time clock handling for reproducible site generation.
//!
//! `SOURCE_DATE_EPOCH`, when present, is the single source of time for a
//! build. Keeping the parsed instant in one value prevents individual output
//! generators from observing different wall-clock times.

use std::ffi::OsStr;

use chrono::{DateTime, Utc};

/// A build's frozen UTC instant and whether it came from reproducible-build
/// metadata.
#[derive(Debug, Clone)]
pub struct BuildTime {
    utc: DateTime<Utc>,
    reproducible: bool,
}

/// Errors produced while reading reproducible-build metadata.
#[derive(Debug, thiserror::Error)]
pub enum BuildTimeError {
    /// The value is intentionally omitted from this message. Environment
    /// values can be attacker-controlled and should not be reflected in logs.
    #[error("invalid SOURCE_DATE_EPOCH")]
    InvalidSourceDateEpoch,
}

impl BuildTime {
    /// Read `SOURCE_DATE_EPOCH`, or capture the current UTC time once when the
    /// variable is absent.
    pub fn from_env() -> Result<Self, BuildTimeError> {
        Self::from_source_date_epoch(std::env::var_os("SOURCE_DATE_EPOCH").as_deref())
    }

    /// Parse a strict, non-negative Unix timestamp.
    ///
    /// Only non-empty ASCII decimal digits are accepted. Signs, whitespace,
    /// fractions, non-Unicode values, overflow, and timestamps outside
    /// Chrono's representable range are rejected.
    pub fn from_source_date_epoch(value: Option<&OsStr>) -> Result<Self, BuildTimeError> {
        let Some(value) = value else {
            return Ok(Self::now());
        };

        let value = value
            .to_str()
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or(BuildTimeError::InvalidSourceDateEpoch)?;
        let seconds = value
            .parse::<u64>()
            .ok()
            .and_then(|seconds| i64::try_from(seconds).ok())
            .ok_or(BuildTimeError::InvalidSourceDateEpoch)?;
        let utc = DateTime::<Utc>::from_timestamp(seconds, 0)
            .ok_or(BuildTimeError::InvalidSourceDateEpoch)?;

        Ok(Self {
            utc,
            reproducible: true,
        })
    }

    /// Capture a non-reproducible wall-clock instant once.
    pub fn now() -> Self {
        Self {
            utc: Utc::now(),
            reproducible: false,
        }
    }

    /// The frozen UTC instant for this build.
    pub fn utc(&self) -> DateTime<Utc> {
        self.utc
    }

    /// Whether the instant was supplied through `SOURCE_DATE_EPOCH`.
    pub fn is_reproducible(&self) -> bool {
        self.reproducible
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn parses_strict_epoch() {
        let build_time = BuildTime::from_source_date_epoch(Some(OsStr::new("946771200"))).unwrap();

        assert!(build_time.is_reproducible());
        assert_eq!(build_time.utc().to_rfc3339(), "2000-01-02T00:00:00+00:00");
    }

    #[test]
    fn rejects_non_canonical_and_out_of_range_values() {
        for value in [
            "",
            "-1",
            "+1",
            "1.0",
            " 1",
            "1 ",
            "9223372036854775808",
            "18446744073709551615",
        ] {
            assert!(BuildTime::from_source_date_epoch(Some(OsStr::new(value))).is_err());
        }
    }
}
