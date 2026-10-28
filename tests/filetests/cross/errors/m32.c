// TARGET: x86_64-unknown-linux-gnu
// C-FLAGS: -m32
// ERROR: check: C flags conflict with the selected target
int main(void) { return 0; }
