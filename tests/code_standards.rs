// The shared file-size gate from CODE_STANDARDS §2.2 — the 800-line hard
// limit on non-test lines — enforced under plain `cargo test`. The client
// (this crate) is the one crate here that depends on the toolkit, so every
// sibling crate in the repo is gated from this file.

fn gate(dir: &str, grandfathered: &[&str]) {
    macroquad_toolkit::source_gate::assert_source_files_within_limit(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir),
        grandfathered,
    );
}

#[test]
fn client_source_files_stay_under_the_limit() {
    gate(".", &[]);
}

#[test]
fn core_source_files_stay_under_the_limit() {
    gate("mytherra-core", &[]);
}

#[test]
fn protocol_source_files_stay_under_the_limit() {
    gate("mytherra-protocol", &[]);
}

#[test]
fn server_source_files_stay_under_the_limit() {
    gate("mytherra-server", &[]);
}

#[test]
fn persistence_source_files_stay_under_the_limit() {
    gate("mytherra-persistence", &[]);
}
