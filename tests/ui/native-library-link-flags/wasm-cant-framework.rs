// the magic command: rustc -Lframework="blah" tests/ui/native-library-link-flags/wasm-cant-framework.rs --target=wasm32-unknown-unknown
// must be a binary, not library
//@ needs-llvm-components: webassembly
fn main() {}
