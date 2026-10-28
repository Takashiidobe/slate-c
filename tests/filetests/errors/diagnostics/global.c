// MODE: error
// LABEL: global
// ERROR: check: label at line
struct Pair { int x; int y; };
struct Pair global = { 1, 2 };
