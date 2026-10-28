// MODE: error
// ERROR: check: intrinsic llvm.smax
// ERROR: check: is not supported
int maximum(int a, int b) {
    return __builtin_elementwise_max(a, b);
}
