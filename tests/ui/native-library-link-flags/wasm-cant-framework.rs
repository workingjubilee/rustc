//@ build-fail
//@ compile-flags: -L framework=blah --target=wasm32-unknown-unknown
//@ needs-llvm-components: webassembly

#![crate_type = "cdylib"] // wasm-pack likes to build everything as a cdylib, so let's emulate it
#![feature(lang_items, no_core)]
#![no_core]
#![no_std]

// some folks set up Nix dev env configs that check for macOS hosts, then pass -Lframework to
// the nixpkgs-controlled Apple SDK (i.e. the one for the host) implicitly.
// linking in a framework for a non-framework-supporting target is erroneous behavior, so
// we reject that, as it is almost always a misconfiguration in the build.
// even for Apple targets, using the host's SDK may be incorrect for deployment targets!

#[lang = "sized"]
trait Sized {}

pub fn blah() {}
