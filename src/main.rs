#[cfg(not(target_arch = "wasm32"))]
fn main() {
    hvat::run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("This binary is not meant to be compiled for WASM. Use the library target instead.");
}
