// TARGET: x86_64-unknown-linux-gnu
// TARGET: x86_64-unknown-linux-musl
// TARGET: aarch64-unknown-linux-gnu
// TARGET: aarch64-unknown-linux-musl
// C-FLAGS: -DVALUE=42 -std=c17 -nostdinc
// CLIF-FLAGS: opt_level=speed
// CLIF: check: iconst.i32 42
int main(void) { return VALUE; }
