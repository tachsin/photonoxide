//! The external libraries found on this machine: `cargo run -p photonoxide-native --example
//! libraries`.

fn main() {
    for probe in photonoxide_native::register_all() {
        println!("{probe}");
    }
}
