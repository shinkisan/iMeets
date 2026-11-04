//! The corpus tests decode menus from captures taken off a real ROM. Those
//! dumps are not part of the public tree, so only build them in when the
//! developer has the fixtures next to the checkout.

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("../../tools/fixtures");
    let needed = [
        "sweets-menu-screens.json",
        "sweets-full-screens.json",
        "large-text-screens.json",
    ];
    println!("cargo:rerun-if-changed=../../tools/fixtures");
    println!("cargo:rustc-check-cfg=cfg(has_fixtures)");
    if needed.iter().all(|f| fixtures.join(f).exists()) {
        println!("cargo:rustc-cfg=has_fixtures");
    }
}
