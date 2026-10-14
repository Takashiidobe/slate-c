#include <string.h>
#include "platform.h"
#include "softfloat.h"
#include "helpers.h"

static extFloat80_t load(const void *source) {
    const unsigned char *bytes = source;
    extFloat80_t value;
    memcpy(&value.signif, bytes, 8);
    memcpy(&value.signExp, bytes + 8, 2);
    if ((value.signif >> 63) && !(value.signExp & 0x7fff)) {
        value.signExp |= 1;
    } else if (!(value.signif >> 63) && (value.signExp & 0x7fff)) {
        softfloat_raiseFlags(softfloat_flag_invalid);
        value.signExp = 0x7fff;
        value.signif = UINT64_C(0xc000000000000000);
    }
    return value;
}

static void store(void *destination, const extFloat80_t *value) {
    unsigned char *bytes = destination;
    memset(bytes, 0, 16);
    memcpy(bytes, &value->signif, 8);
    memcpy(bytes + 8, &value->signExp, 2);
}

#define BINARY(name) \
    void __slate_f80_##name(void *out, const void *left, const void *right) { \
        extFloat80_t a = load(left), b = load(right), result; \
        extF80M_##name(&a, &b, &result); \
        store(out, &result); \
    }

BINARY(add)
BINARY(sub)
BINARY(mul)
BINARY(div)

void __slate_f80_sqrt(void *out, const void *source) {
    extFloat80_t value = load(source), result;
    extF80M_sqrt(&value, &result);
    store(out, &result);
}

void __slate_f80_neg(void *out, const void *source) {
    unsigned char bytes[16];
    memcpy(bytes, source, 10);
    memset(bytes + 10, 0, 6);
    bytes[9] ^= 0x80;
    memcpy(out, bytes, 16);
}

#define COMPARE(name) \
    int32_t __slate_f80_##name(const void *left, const void *right) { \
        extFloat80_t a = load(left), b = load(right); \
        return extF80M_##name(&a, &b); \
    }

COMPARE(eq)
COMPARE(lt)
COMPARE(le)

#define FROM_INTEGER(name, operation, type) \
    void __slate_f80_from_##name(void *out, type value) { \
        extFloat80_t result; \
        operation##_to_extF80M(value, &result); \
        store(out, &result); \
    }

FROM_INTEGER(i64, i64, int64_t)
FROM_INTEGER(u64, ui64, uint64_t)

void __slate_f80_from_f32(void *out, float value) {
    float32_t input;
    extFloat80_t result;
    memcpy(&input.v, &value, 4);
    f32_to_extF80M(input, &result);
    store(out, &result);
}

void __slate_f80_from_f64(void *out, double value) {
    float64_t input;
    extFloat80_t result;
    memcpy(&input.v, &value, 8);
    f64_to_extF80M(input, &result);
    store(out, &result);
}

int64_t __slate_f80_to_i64(const void *source) {
    extFloat80_t value = load(source);
    return extF80M_to_i64_r_minMag(&value, true);
}

uint64_t __slate_f80_to_u64(const void *source) {
    extFloat80_t value = load(source);
    return extF80M_to_ui64_r_minMag(&value, true);
}

float __slate_f80_to_f32(const void *source) {
    extFloat80_t value = load(source);
    float32_t result = extF80M_to_f32(&value);
    float out;
    memcpy(&out, &result.v, 4);
    return out;
}

double __slate_f80_to_f64(const void *source) {
    extFloat80_t value = load(source);
    float64_t result = extF80M_to_f64(&value);
    double out;
    memcpy(&out, &result.v, 8);
    return out;
}

int32_t __slate_f80_set_rounding(uint32_t mode) {
    if (mode > softfloat_round_max) return -1;
    softfloat_roundingMode = mode;
    return 0;
}

uint32_t __slate_f80_get_flags(void) {
    return softfloat_exceptionFlags;
}

void __slate_f80_clear_flags(void) {
    softfloat_exceptionFlags = 0;
}
