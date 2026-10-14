// EXIT: 42
struct Packet { int count; char bytes[]; };
struct Padded { long header; char tag; char bytes[]; };
extern void *malloc(unsigned long size);
extern void free(void *pointer);
int main(void) {
    if (sizeof(struct Packet) != 4) return 1;
    struct Packet *packet = malloc(sizeof(struct Packet) + 2);
    if (!packet) return 2;
    packet->count = 2;
    packet->bytes[0] = 17;
    packet->bytes[1] = 25;
    if ((unsigned long)packet->bytes - (unsigned long)packet != 4) return 3;
    struct Padded *padded = malloc(sizeof(struct Padded) + 2);
    if (!padded) return 4;
    if (sizeof(struct Padded) != 16) return 5;
    if ((unsigned long)padded->bytes - (unsigned long)padded != 9) return 6;
    padded->bytes[0] = 42;
    if (padded->bytes[0] != 42) return 7;
    free(padded);
    int result = packet->bytes[0] + packet->bytes[1];
    free(packet);
    return result;
}
