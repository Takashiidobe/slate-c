// MODE: error
// ERROR: check: union initializers are not supported
int main(void) {
    union Object { int value; long wide; } object = { 42 };
    return object.value;
}
