// Task: Identify and fix the bug in the following function.

// Buggy function: This function is supposed to calculate the factorial of a number.
fn factorial(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn main() {
    println!("Factorial of 5: {}", factorial(5));
}
