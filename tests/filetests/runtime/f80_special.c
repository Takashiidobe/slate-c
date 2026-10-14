// EXIT: 42
struct F80 { unsigned long significand; unsigned short exponent; unsigned short padding[3]; };
extern int __slate_f80_eq(const void *, const void *);
extern int __slate_f80_lt(const void *, const void *);
extern void __slate_f80_div(void *, const void *, const void *);
extern void __slate_f80_neg(void *, const void *);
extern void __slate_f80_clear_flags(void);
extern unsigned int __slate_f80_get_flags(void);
int main(void) {
    struct F80 zero = { 0 };
    struct F80 negative_zero;
    struct F80 one = { 0x8000000000000000UL, 0x3fff };
    struct F80 infinity = { 0x8000000000000000UL, 0x7fff };
    struct F80 nan = { 0xc000000000000000UL, 0x7fff };
    struct F80 pseudo = { 0x8000000000000000UL, 0 };
    struct F80 normal = { 0x8000000000000000UL, 1 };
    struct F80 invalid = { 0x4000000000000000UL, 0x3fff };
    struct F80 result;
    __slate_f80_neg(&negative_zero, &zero);
    if (negative_zero.exponent != 0x8000 || !__slate_f80_eq(&zero, &negative_zero)) return 1;
    __slate_f80_clear_flags();
    __slate_f80_div(&result, &one, &zero);
    if (!__slate_f80_eq(&result, &infinity) || !(__slate_f80_get_flags() & 8)) return 2;
    __slate_f80_clear_flags();
    if (__slate_f80_eq(&nan, &nan) || __slate_f80_get_flags()) return 3;
    if (__slate_f80_lt(&nan, &zero) || !(__slate_f80_get_flags() & 16)) return 4;
    if (!__slate_f80_eq(&pseudo, &normal)) return 5;
    __slate_f80_clear_flags();
    if (__slate_f80_eq(&invalid, &one) || !(__slate_f80_get_flags() & 16)) return 6;
    return 42;
}
