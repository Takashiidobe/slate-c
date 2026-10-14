// MODE: error
// ERROR: check: pointer subtraction with function type is not supported
int main(void) {
    int (*left)(void) = 0;
    int (*right)(void) = 0;
    return left - right;
}
