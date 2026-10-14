// EXIT: 42
struct Inner { char tag; long value; };
typedef struct Inner Inner;
typedef Inner InnerAlias;
union Data { long value; char bytes[8]; };
struct Outer { char tag; InnerAlias inner; union Data data; char end; };
typedef struct Outer Outer;
Outer global[2];
int main(void) {
    Outer local[2];
    if (sizeof(InnerAlias) != 16 || _Alignof(InnerAlias) != 8) return 1;
    if (sizeof(Outer) != 40 || _Alignof(Outer) != 8) return 2;
    if ((unsigned long)&local[0].inner - (unsigned long)&local[0] != 8) return 3;
    if ((unsigned long)&local[0].data - (unsigned long)&local[0] != 24) return 4;
    if ((unsigned long)&local[1] - (unsigned long)&local[0] != 40) return 5;
    local[1].inner.value = 17;
    local[1].data.value = 25;
    global[1] = local[1];
    if ((unsigned long)&global[1].data.value != (unsigned long)&global[1].data.bytes) return 6;
    return global[1].inner.value + global[1].data.bytes[0];
}
