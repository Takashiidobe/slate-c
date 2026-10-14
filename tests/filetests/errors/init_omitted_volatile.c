// MODE: error
// ERROR: check: atomic and volatile initialization is not supported
int main(void) {
    struct Object { int tag; volatile int value; } object = { 1 };
    return 0;
}
