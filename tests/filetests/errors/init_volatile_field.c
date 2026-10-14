// MODE: error
// ERROR: check: atomic and volatile initialization is not supported
int main(void) {
    struct Object { volatile int value; int tag; } object = { 0 };
    return 0;
}
