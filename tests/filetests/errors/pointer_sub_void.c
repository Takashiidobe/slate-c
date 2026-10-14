// MODE: error
// ERROR: check: pointer subtraction with void type is not supported
int main(void) {
    void *left = 0;
    void *right = 0;
    return left - right;
}
