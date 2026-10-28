// MODE: error
// ERROR: check: runtime lowering for builtin __builtin_exp is not supported
double exponential(double value) {
    return __builtin_exp(value);
}
