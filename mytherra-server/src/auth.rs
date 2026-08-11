//! WebHatchery account-token verification (GDD 7.3).
//!
//! Account linking reuses the existing shared auth wholesale (§7.3): the
//! WebHatchery login issues an HS256 JWT signed with a secret every backend
//! shares, and a client presents that token to bind — or later resume — its
//! deity. This module is the Rust counterpart to the original PHP backend's
//! `firebase/php-jwt` check: it validates the signature and expiry against the
//! same `JWT_SECRET` and hands back the authenticated account identity. It never
//! *mints* tokens (the login owns that) and does not touch the database.

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

/// A verified, signed-in WebHatchery account — the identity a deity links to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The account's stable id (the token's `sub`), the value stored in
    /// `players.account_id`.
    pub id: String,
}

/// Why a presented token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    /// Missing, malformed, wrong-signature, or expired — not a valid token.
    Invalid,
    /// A well-formed *guest* token. Guests can play, but linking and resuming a
    /// persistent deity require a real, signed-in account (§7.3).
    Guest,
}

/// The claims we read from a WebHatchery token. Registered claims like `exp` are
/// validated by `jsonwebtoken` itself, so only the identity fields appear here.
#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    is_guest: bool,
}

/// Verify a presented bearer token against the shared secret and return the
/// account it authenticates (GDD 7.3). A guest token is rejected with
/// [`AuthError::Guest`] — it is valid, but not an account to bind to.
pub fn verify_account(token: &str, secret: &str) -> Result<Account, AuthError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    // HS256 with default validation: signature, and `exp` (required + not past).
    let validation = Validation::new(Algorithm::HS256);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| AuthError::Invalid)?
        .claims;
    if claims.is_guest {
        return Err(AuthError::Guest);
    }
    if claims.sub.trim().is_empty() {
        return Err(AuthError::Invalid);
    }
    Ok(Account { id: claims.sub })
}

#[cfg(test)]
mod tests;
