#ifndef SLATE_F80_HELPERS_H
#define SLATE_F80_HELPERS_H

#include <stdint.h>

void __slate_f80_add(void *, const void *, const void *);
void __slate_f80_sub(void *, const void *, const void *);
void __slate_f80_mul(void *, const void *, const void *);
void __slate_f80_div(void *, const void *, const void *);
void __slate_f80_sqrt(void *, const void *);
void __slate_f80_neg(void *, const void *);
int32_t __slate_f80_eq(const void *, const void *);
int32_t __slate_f80_lt(const void *, const void *);
int32_t __slate_f80_le(const void *, const void *);
void __slate_f80_from_i64(void *, int64_t);
void __slate_f80_from_u64(void *, uint64_t);
void __slate_f80_from_f32(void *, float);
void __slate_f80_from_f64(void *, double);
int64_t __slate_f80_to_i64(const void *);
uint64_t __slate_f80_to_u64(const void *);
float __slate_f80_to_f32(const void *);
double __slate_f80_to_f64(const void *);
int32_t __slate_f80_set_rounding(uint32_t);
uint32_t __slate_f80_get_flags(void);
void __slate_f80_clear_flags(void);

#endif
