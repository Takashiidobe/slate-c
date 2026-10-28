// TARGET: x86_64-unknown-linux-gnu
// TARGET: x86_64-unknown-linux-musl
// TARGET: aarch64-unknown-linux-gnu
// TARGET: aarch64-unknown-linux-musl
// FUNCTIONS: 1
int global = 7; int other(int x) { return global + x; }
