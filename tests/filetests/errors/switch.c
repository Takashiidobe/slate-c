// MODE: error
// ERROR: check: switch statements are not supported
int main(void) {
    switch (1) { case 1: return 42; default: return 0; }
}
