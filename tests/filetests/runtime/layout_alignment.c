// EXIT: 42
typedef unsigned long Address;
struct __attribute__((aligned(64))) Wide { int value; };
_Alignas(64) char byte;
_Alignas(128) int number = 17;
struct Wide wide;
char large_array[17];
int main(void) {
    _Alignas(64) char local_byte;
    _Alignas(128) int local_number = 25;
    struct Wide local_wide;
    char local_array[17];
    if ((Address)&byte % 64) return 1;
    if ((Address)&number % 128) return 2;
    if ((Address)&wide % 64) return 3;
    if ((Address)&local_byte % 64) return 4;
    if ((Address)&local_number % 128) return 5;
    if ((Address)&local_wide % 64) return 6;
    if ((Address)local_array % 16) return 7;
    if ((Address)large_array % 16) return 8;
    local_wide.value = number;
    wide = local_wide;
    return wide.value + local_number;
}
