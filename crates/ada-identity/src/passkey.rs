//! Passkey = WebAuthn resident-key flow. Re-export of the
//! registration ceremony with discoverable_credential flag.

pub use crate::webauthn::{begin_registration as begin_passkey_registration, RegistrationChallenge as PasskeyChallenge};