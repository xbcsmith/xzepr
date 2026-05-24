// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Internal macro for generating ULID-backed domain identifier value objects.
//!
//! Use [`define_ulid_id`] to define a new identifier type with a consistent
//! API, serde support, SQLx Postgres integration, and full test coverage.

/// Generate a ULID-backed domain identifier value object.
///
/// This macro expands to a complete newtype struct wrapping a `ulid::Ulid`,
/// with the following implementations automatically provided:
///
/// - `new()` -- creates a fresh identifier
/// - `from_ulid(ulid: Ulid) -> Self`
/// - `parse(s: &str) -> Result<Self, ulid::DecodeError>`
/// - `from_string(s: String) -> Result<Self, ulid::DecodeError>`
/// - `as_ulid() -> Ulid`
/// - `as_str() -> String`
/// - `timestamp_ms() -> u64`
/// - `Default`, `Display`, `From<Ulid>`, `From<Self> for Ulid`, `FromStr`
/// - `sqlx::Type`, `sqlx::Encode`, `sqlx::Decode` for `sqlx::Postgres`
///
/// # Examples
///
/// ```rust
/// // Define a new identifier type called `WidgetId`
/// xzepr::define_ulid_id!(WidgetId, "Unique identifier for a Widget.");
///
/// let id = WidgetId::new();
/// let s = id.to_string();
/// let parsed: WidgetId = s.parse().unwrap();
/// assert_eq!(id, parsed);
/// ```
#[macro_export]
macro_rules! define_ulid_id {
    // Variant with explicit doc string
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize,
        )]
        pub struct $name(::ulid::Ulid);

        impl $name {
            /// Creates a new identifier backed by a freshly-generated ULID.
            pub fn new() -> Self {
                Self(::ulid::Ulid::new())
            }

            /// Creates an identifier from an existing `Ulid`.
            pub fn from_ulid(ulid: ::ulid::Ulid) -> Self {
                Self(ulid)
            }

            /// Parses an identifier from its canonical string representation.
            ///
            /// # Errors
            ///
            /// Returns `ulid::DecodeError` if `s` is not a valid ULID string.
            pub fn parse(s: &str) -> Result<Self, ::ulid::DecodeError> {
                Ok(Self(::ulid::Ulid::from_string(s)?))
            }

            /// Parses an identifier from an owned string (alias for `parse`).
            ///
            /// # Errors
            ///
            /// Returns `ulid::DecodeError` if the string is not a valid ULID.
            pub fn from_string(s: String) -> Result<Self, ::ulid::DecodeError> {
                Self::parse(&s)
            }

            /// Returns the underlying `Ulid`.
            pub fn as_ulid(&self) -> ::ulid::Ulid {
                self.0
            }

            /// Returns the canonical string representation of this identifier.
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }

            /// Returns the millisecond-precision Unix timestamp embedded in the ULID.
            pub fn timestamp_ms(&self) -> u64 {
                self.0.timestamp_ms()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<::ulid::Ulid> for $name {
            fn from(ulid: ::ulid::Ulid) -> Self {
                Self(ulid)
            }
        }

        impl From<$name> for ::ulid::Ulid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::ulid::DecodeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        // SQLx Postgres support
        impl ::sqlx::Type<::sqlx::Postgres> for $name {
            fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                <String as ::sqlx::Type<::sqlx::Postgres>>::type_info()
            }
        }

        impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for $name {
            fn decode(
                value: ::sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, ::sqlx::error::BoxDynError> {
                let s = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                Ok(Self::parse(&s)?)
            }
        }

        impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut ::sqlx::postgres::PgArgumentBuffer,
            ) -> Result<::sqlx::encode::IsNull, Box<dyn ::std::error::Error + Send + Sync>> {
                <String as ::sqlx::Encode<::sqlx::Postgres>>::encode_by_ref(&self.to_string(), buf)
            }
        }
    };

    // Variant without explicit doc string (uses a generated one)
    ($name:ident) => {
        $crate::define_ulid_id!(
            $name,
            concat!("ULID-backed unique identifier: `", stringify!($name), "`.")
        );
    };
}

#[cfg(test)]
mod tests {
    // Verify the macro compiles and produces correct behaviour for a local test type.
    crate::define_ulid_id!(TestId, "Identifier used only in tests.");

    #[test]
    fn test_new_generates_unique_ids() {
        let a = TestId::new();
        let b = TestId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_roundtrip_via_string() {
        let id = TestId::new();
        let s = id.to_string();
        // SAFETY: test data is a freshly-generated valid ULID string
        let parsed: TestId = s.parse().expect("valid ULID string should parse");
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_from_string_alias() {
        let id = TestId::new();
        let s = id.as_str();
        // SAFETY: test data is a freshly-generated valid ULID string
        let parsed = TestId::from_string(s).expect("valid ULID string should parse");
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_parse_invalid_returns_error() {
        assert!(TestId::parse("not-a-ulid").is_err());
    }

    #[test]
    fn test_default_is_non_empty() {
        let id = TestId::default();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn test_from_ulid_roundtrip() {
        let ulid = ::ulid::Ulid::new();
        let id = TestId::from_ulid(ulid);
        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn test_into_ulid() {
        let id = TestId::new();
        // TestId is Copy, so id remains valid after the Into conversion
        let ulid: ::ulid::Ulid = id.into();
        assert_eq!(ulid, id.as_ulid());
    }

    #[test]
    fn test_timestamp_ms_in_range() {
        let id = TestId::new();
        let ts = id.timestamp_ms();
        assert!(ts > 1_577_836_800_000); // after 2020-01-01
        assert!(ts < 2_000_000_000_000); // before ~2033
    }

    #[test]
    fn test_serde_roundtrip() {
        let id = TestId::new();
        // SAFETY: TestId serializes to a ULID string which is always valid JSON
        let json = serde_json::to_string(&id).expect("serialize");
        // SAFETY: we just serialized this value, it must deserialize cleanly
        let back: TestId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn test_display_equals_as_str() {
        let id = TestId::new();
        assert_eq!(id.to_string(), id.as_str());
    }
}
