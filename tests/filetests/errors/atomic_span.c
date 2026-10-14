// MODE: error
// ERROR: check: atomic and volatile access is not supported
// ERROR: check: label at line
// ERROR: check: lowering failed here
// ERROR: check: diagnostic code: slate_c::lowering
int main(void) {
    volatile int value;
    value = 1;
    return value;
}
