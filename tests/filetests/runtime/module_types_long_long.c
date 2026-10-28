// EXIT: 19
typedef long long Number;
struct Pair { Number x; Number y; };
struct Pair global;
int main(void) {
    struct Pair values[2];
    values[1].x = 1;
    global.y = 2;
    return values[1].x + global.y + sizeof(struct Pair);
}
