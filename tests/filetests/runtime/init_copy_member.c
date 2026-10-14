// EXIT: 42
union Value { long value; unsigned char bytes[8]; };
struct Object { union Value value; int tag; };
int main(void) {
    union Value original;
    original.value = 17;
    struct Object object = { original, 25 };
    original.value = 100;
    return object.value.value + object.tag;
}
