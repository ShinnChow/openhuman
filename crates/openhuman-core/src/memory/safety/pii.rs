//! Multilingual national-ID and personal-PII redaction.
//!
//! Split out of the original `safety.rs` (see [`super`]'s doc comment for
//! why this policy lives in OpenHuman rather than the memory engine).
//! Three responsibilities, three files:
//!
//! - [`checksums`] — Luhn / IBAN mod-97 / Verhoeff / CPF / CNPJ / CUIT /
//!   Spanish DNI-NIE structural validators, with no regex or normalization
//!   dependency.
//! - [`normalize`] — the Unicode normalization pass and the cheap
//!   byte-oriented candidate pre-filter that decides which precise regexes
//!   are even worth running.
//! - [`patterns`] — the per-identifier regexes, candidate-gated match
//!   collection, and the `redact_pii`/`has_likely_pii`/`has_likely_email`
//!   entry points this module re-exports.
//!
//! The generic secret-pattern scrubber and the `Sanitized`/
//! `SanitizationReport` types live in [`super::secrets`].

mod checksums;
mod normalize;
mod patterns;

pub(super) use patterns::{has_likely_email, has_likely_pii, redact_pii};
