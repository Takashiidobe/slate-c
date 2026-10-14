// EXIT: 42
struct F80 { unsigned long significand; unsigned short exponent; unsigned short padding[3]; };
extern int __slate_f80_set_rounding(unsigned int);
extern void __slate_f80_clear_flags(void);
extern unsigned int __slate_f80_get_flags(void);
extern double __slate_f80_to_f64(const void *);
extern float __slate_f80_to_f32(const void *);
int main(void) {
    struct F80 halfway = { 0x8000000000000400UL, 0x3fff };
    struct F80 single_halfway = { 0x8000008000000000UL, 0x3fff };
    if (__slate_f80_set_rounding(0)) return 1;
    __slate_f80_clear_flags();
    if (__slate_f80_to_f64(&halfway) != 1.0) return 2;
    if (!(__slate_f80_get_flags() & 1)) return 3;
    if (__slate_f80_set_rounding(3)) return 4;
    if (__slate_f80_to_f64(&halfway) != 0x1.0000000000001p0) return 5;
    if (__slate_f80_to_f32(&single_halfway) != 0x1.000002p0f) return 6;
    if (__slate_f80_set_rounding(99) != -1) return 7;
    if (__slate_f80_to_f64(&halfway) != 0x1.0000000000001p0) return 8;
    __slate_f80_clear_flags();
    if (__slate_f80_get_flags()) return 9;
    __slate_f80_set_rounding(0);
    return 42;
}
