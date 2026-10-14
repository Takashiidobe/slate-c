// EXIT: 42
enum Count { FIRST = 17, SECOND = 25 };
typedef enum Count Count;
typedef Count CountAlias;
CountAlias values[2];
int main(void) {
    CountAlias local[2];
    if (sizeof(CountAlias) != 4 || _Alignof(CountAlias) != 4) return 1;
    if ((unsigned long)&local[1] - (unsigned long)&local[0] != 4) return 2;
    local[0] = FIRST;
    values[1] = SECOND;
    return local[0] + values[1];
}
