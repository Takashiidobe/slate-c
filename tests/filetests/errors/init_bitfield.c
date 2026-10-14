// MODE: error
// ERROR: check: bit-field initializers are not supported
// ERROR: check: lowering failed here
int main(void) {
    struct Bits { unsigned value : 3; } bits = { 1 };
    return 0;
}
