//! The external libraries found on this machine, and the backends registered from them:
//! `cargo run -p photonoxide-native --example libraries`.

fn main() {
    for probe in photonoxide_native::register_all() {
        println!("{probe}");
    }
    // the backends, after their smoke tests: available, or why not
    let direct = photonoxide::backend::direct_solvers().unwrap_or_default();
    let iterative = photonoxide::backend::iterative_solvers().unwrap_or_default();
    for (kind, listed) in [("direct", direct), ("iterative", iterative)] {
        for backend in listed {
            match (backend.capabilities, backend.unavailable) {
                (Some(c), _) => println!("backend {} ({kind}): {} available", c.name, c.version),
                (None, Some(why)) => {
                    println!("backend {} ({kind}): unavailable: {why}", backend.name)
                }
                (None, None) => println!("backend {} ({kind})", backend.name),
            }
        }
    }
}
