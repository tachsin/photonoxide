//! The studio window's capabilities (capabilities/default.json) let it open the links it shows:
//! the opener plugin's `allow-open-url` comes with no scope of its own, so without a scope (its
//! `allow-default-urls`, `http`, `https`, `mailto` and `tel`) every link is refused, and the
//! vendors' downloads, the papers' DOIs and the licences go nowhere.

use std::path::Path;

#[test]
fn the_window_may_open_web_and_mail_links() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json");
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("default.json is JSON");
    let permissions: Vec<&str> = json["permissions"]
        .as_array()
        .expect("a permissions list")
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    let opens = permissions.contains(&"opener:default")
        || (permissions.contains(&"opener:allow-open-url")
            && permissions.contains(&"opener:allow-default-urls"));
    assert!(
        opens,
        "the window can't open links: capabilities/default.json needs opener:allow-default-urls \
         beside opener:allow-open-url (or opener:default); it has {permissions:?}"
    );
}
