// EXIT: 0
double root(double value) { return __builtin_sqrt(value); }
float rootf(float value) { return __builtin_sqrtf(value); }
double sine(double value) { return __builtin_sin(value); }
double cosine(double value) { return __builtin_cos(value); }
double fused(double a, double b, double c) { return __builtin_fma(a, b, c); }
float fusedf(float a, float b, float c) { return __builtin_fmaf(a, b, c); }
int main(void) {
    if (root(81.0) != 9.0 || rootf(16.0f) != 4.0f) return 1;
    if (sine(0.0) != 0.0 || cosine(0.0) != 1.0) return 2;
    if (fused(0x1.0000000000001p0, 0x1.ffffffffffffep-1, -1.0) != -0x1p-104) return 3;
    return fusedf(2.0f, 3.0f, 4.0f) != 10.0f;
}
