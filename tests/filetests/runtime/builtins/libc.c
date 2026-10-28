// EXIT: 0
int main(void) {
    char source[4] = { 1, 2, 3, 4 };
    char dest[4];
    if (__builtin_memset(dest, 0, 4) != dest) return 1;
    if (__builtin_memcpy(dest, source, 4) != dest) return 2;
    if (__builtin_memcmp(dest, source, 4) != 0) return 3;
    if (__builtin_memmove(dest + 1, dest, 3) != dest + 1) return 4;
    return dest[0] != 1 || dest[1] != 1 || dest[2] != 2 || dest[3] != 3;
}
