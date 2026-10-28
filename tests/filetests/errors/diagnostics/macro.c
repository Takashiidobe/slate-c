// MODE: error
// DIAGNOSTIC: lowering
// LABEL: SELECT
// ERROR: check: label at line
#define SELECT switch
int main(void) { SELECT (1) { default: return 0; } }
