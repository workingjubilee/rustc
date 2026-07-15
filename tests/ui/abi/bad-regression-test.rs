//@ run-pass
use std::hint::black_box;

#[repr(C)]
struct Bools {
    a: bool,
    b: bool,
    c: bool,
    d: bool,
}

#[link(name = "get_a")]
unsafe extern "C" {
    safe fn get_a(bools: Bools) -> bool;
}

pub fn broken(arr: &[i32]) -> i32 {
    if arr.len() < 2 {
        return 0;
    }
    let bools = Bools {
        a: false,
        b: true,
        c: false,
        d: true,
    };
    let ret = get_a(bools);
    arr[ret as usize]
}

fn main() {
    let ret = black_box(broken(&*Box::new([0, 1])));
    assert_eq!(ret, 0);
}
