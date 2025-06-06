fn system(f: extern "system" fn(usize, ...)) {
    //~^  ERROR using calling conventions other than "C"

    f(22, 44);
}

fn main() {}
