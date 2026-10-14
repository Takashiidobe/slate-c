// EXIT: 42
struct F80 { unsigned long significand; unsigned short exponent; unsigned short padding[3]; };
extern void __slate_f80_add(void *, const void *, const void *);
extern void __slate_f80_sub(void *, const void *, const void *);
extern void __slate_f80_mul(void *, const void *, const void *);
extern int __slate_f80_eq(const void *, const void *);
extern void __slate_f80_from_u64(void *, unsigned long);
extern unsigned long __slate_f80_to_u64(const void *);
extern long __slate_f80_to_i64(const void *);
int main(void) {
    _Alignas(16) struct F80 one = { 0x8000000000000000UL, 0x3fff };
    _Alignas(16) struct F80 tiny = { 0x8000000000000000UL, 0x3fc0 };
    _Alignas(16) struct F80 scale = { 0x8000000000000000UL, 0x403e };
    _Alignas(16) struct F80 result;
    __slate_f80_add(&result, &one, &tiny);
    if (result.significand != 0x8000000000000001UL || result.exponent != 0x3fff) return 1;
    if (result.padding[0] || result.padding[1] || result.padding[2]) return 2;
    __slate_f80_sub(&result, &result, &one);
    if (!__slate_f80_eq(&result, &tiny)) return 3;
    __slate_f80_mul(&result, &result, &scale);
    if (__slate_f80_to_i64(&result) != 1) return 4;
    __slate_f80_from_u64(&result, 9007199254740993UL);
    if (__slate_f80_to_u64(&result) != 9007199254740993UL) return 5;
    return 42;
}
