// MODE: compile
// CLIF: check: ; forward
// CLIF: check: call $func()
// CLIF: nextln: return
// CLIF: not: iconst
extern void action(void);
void forward(void) { return action(); }
