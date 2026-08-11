use super::*;
use std::time::{Duration, Instant};

/// Poll a request to completion (native tests only — quad-net resolves it on
/// a background thread). Never used in the game loop, which polls per frame.
fn block_on<T: DeserializeOwned>(mut pending: Pending<T>) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = pending.poll() {
            return result;
        }
        if Instant::now() > deadline {
            return Err("timed out waiting for the server".to_owned());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// End-to-end against a running `mytherra-server` on the default port. Start
/// the server, then run: `cargo test -p mytherra -- --ignored net`.
#[test]
#[ignore = "needs a live mytherra-server on 127.0.0.1:8791"]
fn round_trip_against_a_live_server() {
    let mut client = ServerClient::new("http://127.0.0.1:8791");

    // A request without a session is rejected (§7.7).
    assert!(
        block_on(client.fetch_view()).is_err(),
        "a view without a session is unauthorized"
    );

    // Establish a guest session; every later request carries its id.
    let session = block_on(client.create_session()).expect("create session");
    client.set_player_id(session.player_id);

    let view = block_on(client.fetch_view()).expect("fetch view");
    assert!(!view.world.heroes.is_empty(), "a fresh guest sees heroes");
    assert!(
        view.world.regions.is_empty(),
        "a Watcher has not unlocked regions"
    );

    // A Watcher may designate a champion (hero-adjacent, §5.9). Pick a *living*
    // hero — in a long-running world most of the roster has passed on, and a
    // champion can only be raised from the quick.
    let hero = view
        .world
        .heroes
        .iter()
        .find(|h| h.is_alive)
        .expect("a living hero to champion")
        .id
        .clone();
    let report = block_on(client.submit_action(&PlayerAction::DesignateChampion { hero_id: hero }))
        .expect("designate champion");
    assert!(!report.feedback.is_empty(), "the act reports feedback");

    // ...but a region action is forbidden at Watcher standing (§7.7).
    let forbidden = block_on(client.submit_action(&PlayerAction::RegionAction {
        region_id: "aldermoor".to_owned(),
        action_id: "bless".to_owned(),
    }));
    assert!(forbidden.is_err(), "regions are locked at Watcher standing");

    let delta = block_on(client.fetch_events(0)).expect("fetch events");
    assert!(delta.cursor >= 1, "the awakening event advances the cursor");
}

/// Account linking end-to-end from the *client's* side (GDD 7.3): a guest
/// links its deity to a WebHatchery account, and a fresh client presenting the
/// same token resumes the very same deity. Needs a live server whose
/// `JWT_SECRET` matches `SECRET` below. Run:
/// `cargo test -p mytherra -- --ignored account_link`.
#[test]
#[ignore = "needs a live mytherra-server on 127.0.0.1:8791 with the dev JWT_SECRET"]
fn account_link_then_resume_across_clients() {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    // Must match the server's dev `mytherra-server/.env` JWT_SECRET.
    const SECRET: &str = "mytherra-dev-shared-secret-local-only";
    #[derive(serde::Serialize)]
    struct Claims {
        sub: String,
        is_guest: bool,
        exp: usize,
    }
    let token = encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: "wh-net-test".to_owned(),
            is_guest: false,
            exp: 4_102_444_800, // 2100
        },
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .expect("forge account token");

    let base = "http://127.0.0.1:8791";

    // A guest links the deity it is playing to the account.
    let mut guest = ServerClient::new(base);
    let gid = block_on(guest.create_session())
        .expect("guest session")
        .player_id;
    guest.set_player_id(gid);
    guest.set_token(Some(token.clone()));
    let linked = block_on(guest.link()).expect("link");
    assert!(linked.linked, "the deity is now account-bound");

    // A fresh client presenting the same token resumes the SAME deity — the
    // heart of cross-device continuity.
    let mut returning = ServerClient::new(base);
    returning.set_token(Some(token));
    let resumed = block_on(returning.create_session()).expect("resume");
    assert!(resumed.linked, "the resumed deity is account-bound");
    assert_eq!(
        resumed.player_id, linked.player_id,
        "the same god is resumed across clients"
    );
}

/// Two guests get independent state: each has its own favor, so one deity's
/// spending never touches another's. Start the server, then run:
/// `cargo test -p mytherra -- --ignored two_guests`.
#[test]
#[ignore = "needs a live mytherra-server on 127.0.0.1:8791"]
fn two_guests_hold_independent_favor() {
    let base = "http://127.0.0.1:8791";
    let mut alice = ServerClient::new(base);
    let mut bob = ServerClient::new(base);
    alice.set_player_id(
        block_on(alice.create_session())
            .expect("alice session")
            .player_id,
    );
    bob.set_player_id(
        block_on(bob.create_session())
            .expect("bob session")
            .player_id,
    );

    let alice_before = block_on(alice.fetch_view())
        .expect("alice view")
        .player
        .player
        .favor;
    let bob_before = block_on(bob.fetch_view())
        .expect("bob view")
        .player
        .player
        .favor;

    // Alice designates a champion — favor leaves *her* purse. Champions are
    // raised only from living heroes, rare in a long-lived world.
    let hero = block_on(alice.fetch_view())
        .expect("alice view")
        .world
        .heroes
        .iter()
        .find(|h| h.is_alive)
        .expect("a living hero")
        .id
        .clone();
    block_on(alice.submit_action(&PlayerAction::DesignateChampion { hero_id: hero }))
        .expect("alice designates");

    let alice_after = block_on(alice.fetch_view())
        .expect("alice view")
        .player
        .player
        .favor;
    let bob_after = block_on(bob.fetch_view())
        .expect("bob view")
        .player
        .player
        .favor;

    assert!(alice_after < alice_before, "alice paid for her champion");
    assert_eq!(bob_after, bob_before, "bob's favor is untouched by alice");
}
