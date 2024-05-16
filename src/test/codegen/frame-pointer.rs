// compile-flags: --crate-type=rlib -Copt-level=0
// revisions: force-on aarch64-apple aarch64-apple-off
// [aarch64-apple] needs-llvm-components: aarch64
// [aarch64-apple] compile-flags: --target=aarch64-apple-darwin
// [aarch64-apple-off] needs-llvm-components: aarch64
// [aarch64-apple-off] compile-flags: --target=aarch64-apple-darwin -Cforce-frame-pointers=off
// [force-on] needs-llvm-components: x86
// [force-on] compile-flags: --target=x86_64-unknown-linux-gnu -Cforce-frame-pointers=on
#![feature(no_core, lang_items)]
#![no_core]
#[lang = "sized"]
trait Sized {}
#[lang = "copy"]
trait Copy {}
impl Copy for u32 {}

// CHECK: define i32 @peach{{.*}}[[PEACH_ATTRS:\#[0-9]+]] {
#[no_mangle]
pub fn peach(x: u32) -> u32 {
    x
}

// CHECK: attributes [[PEACH_ATTRS]] = {
// force-on-SAME: {{.*}}"frame-pointer"="all"
// aarch64-apple-SAME: {{.*}}"frame-pointer"="all"
// aarch64-apple-off-NOT: {{.*}}"frame-pointer"{{.*}}
// CHECK-SAME: }
