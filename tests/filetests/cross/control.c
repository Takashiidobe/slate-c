// TARGET: x86_64-unknown-linux-gnu
// TARGET: x86_64-unknown-linux-musl
// TARGET: aarch64-unknown-linux-gnu
// TARGET: aarch64-unknown-linux-musl
// FUNCTIONS: 2
// CLIF: check: call
int add(int a, int b) { return a + b; }
int main(void) {
    int sum = 0;
    for (int i = 0; i < 5; i++) sum = add(sum, i);
    return sum;
}
