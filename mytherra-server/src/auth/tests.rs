use super::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;

const SECRET: &str = "shared-webhatchery-test-secret";

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    is_guest: bool,
    exp: usize,
}

fn token(secret: &str, sub: &str, is_guest: bool, exp: usize) -> String {
    let claims = TestClaims {
        sub: sub.to_owned(),
        is_guest,
        exp,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

// A fixed far-future expiry; the crate forbids `Date`/time in this workspace
// and these tests never span decades of real wall-clock.
const FAR_FUTURE: usize = 4_102_444_800; // 2100-01-01

#[test]
fn a_real_account_token_authenticates_its_holder() {
    let account = verify_account(&token(SECRET, "wh-user-42", false, FAR_FUTURE), SECRET);
    assert_eq!(
        account,
        Ok(Account {
            id: "wh-user-42".to_owned()
        })
    );
}

#[test]
fn a_guest_token_is_valid_but_not_an_account() {
    let result = verify_account(&token(SECRET, "guest_abc", true, FAR_FUTURE), SECRET);
    assert_eq!(result, Err(AuthError::Guest));
}

#[test]
fn a_token_signed_with_another_secret_is_rejected() {
    let forged = token("some-other-secret", "wh-user-42", false, FAR_FUTURE);
    assert_eq!(verify_account(&forged, SECRET), Err(AuthError::Invalid));
}

#[test]
fn an_expired_token_is_rejected() {
    // Expired in 1970; validation's small default leeway can't save it.
    assert_eq!(
        verify_account(&token(SECRET, "wh-user-42", false, 1), SECRET),
        Err(AuthError::Invalid)
    );
}

#[test]
fn garbage_is_rejected() {
    assert_eq!(verify_account("not.a.jwt", SECRET), Err(AuthError::Invalid));
    assert_eq!(verify_account("", SECRET), Err(AuthError::Invalid));
}
