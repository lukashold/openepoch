//! Explicit local-only inputs for ignored authored-map integration checks.
//! No original payload is embedded, searched for implicitly or distributed.
pub(crate) fn read(name: &str) -> Vec<u8> {
    assert!(matches!(name, "world.w8in" | "world0.dvp1" | "animal-manager.anm"));
    let root = std::env::var_os("OPENEPOCH_PRIVATE_FIXTURES")
        .expect("Set OPENEPOCH_PRIVATE_FIXTURES to your local original input directory before running ignored integration tests");
    std::fs::read(std::path::PathBuf::from(root).join(name))
        .expect("Explicit local original fixture missing; no fixture bytes are included in the public repository")
}
