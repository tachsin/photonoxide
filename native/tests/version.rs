//! photonoxide-native has photonoxide's version: its crate takes the workspace's.

#[test]
fn the_crate_has_photonoxide_s_version() {
    assert_eq!(env!("CARGO_PKG_VERSION"), photonoxide::VERSION);
}
