// EXIT: 42
union Word { unsigned long word; unsigned char bytes[8]; };
struct Object { union Word value; unsigned char tag; };
int main(void) {
    struct Object original;
    original.value.word = 17;
    original.tag = 25;
    struct Object copy = original;
    original.value.bytes[0] = 99;
    union Word word = copy.value;
    return word.bytes[0] + copy.tag;
}
