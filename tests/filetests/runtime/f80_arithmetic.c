// EXIT: 42
struct F80 { unsigned long significand; unsigned short exponent; unsigned short padding[3]; };
extern void __slate_f80_from_i64(void *, long);
extern void __slate_f80_from_f32(void *, float);
extern void __slate_f80_from_f64(void *, double);
extern void __slate_f80_add(void *, const void *, const void *);
extern void __slate_f80_mul(void *, const void *, const void *);
extern void __slate_f80_div(void *, const void *, const void *);
extern void __slate_f80_sqrt(void *, const void *);
extern void __slate_f80_neg(void *, const void *);
extern long __slate_f80_to_i64(const void *);
extern double __slate_f80_to_f64(const void *);
extern float __slate_f80_to_f32(const void *);
extern int __slate_f80_lt(const void *, const void *);
extern int __slate_f80_le(const void *, const void *);
int main(void) {
    struct F80 left;
    struct F80 right;
    struct F80 result;
    __slate_f80_from_i64(&left, 9);
    __slate_f80_sqrt(&result, &left);
    if (__slate_f80_to_i64(&result) != 3) return 1;
    __slate_f80_from_f32(&left, 3.5f);
    __slate_f80_from_f64(&right, 2.0);
    __slate_f80_mul(&result, &left, &right);
    __slate_f80_div(&result, &result, &right);
    if (__slate_f80_to_f32(&result) != 3.5f) return 2;
    if (__slate_f80_to_f64(&result) != 3.5) return 3;
    __slate_f80_neg(&result, &result);
    if (__slate_f80_to_i64(&result) != -3) return 4;
    if (!__slate_f80_lt(&result, &left) || !__slate_f80_le(&left, &left)) return 5;
    __slate_f80_from_i64(&left, 17);
    __slate_f80_from_i64(&right, 25);
    __slate_f80_add(&result, &left, &right);
    return __slate_f80_to_i64(&result);
}
