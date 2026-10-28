// EXIT: 0
// CLIF: check: $(fused=$var) = fma $var, $var, $var
// CLIF: check: return $fused
double fused(double a, double b, double c) {
    return __builtin_elementwise_fma(a, b, c);
}
int main(void) {
    return fused(0x1.0000000000001p0, 0x1.ffffffffffffep-1, -1.0) != -0x1p-104;
}
