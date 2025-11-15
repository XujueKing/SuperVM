#![no_std]
#![no_main]

use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

fn main() {
    let (mut a0, mut a1, rounds): (u64, u64, u32) = env::read();
    for _ in 0..rounds {
        let next = a0.wrapping_add(a1);
        a0 = a1;
        a1 = next;
    }
    env::commit(&a1);
}
