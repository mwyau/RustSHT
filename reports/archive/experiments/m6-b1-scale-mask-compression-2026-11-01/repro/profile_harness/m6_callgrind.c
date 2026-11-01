#include <valgrind/callgrind.h>
void m6_callgrind_zero(void) { CALLGRIND_ZERO_STATS; }
void m6_callgrind_start(void) {
  CALLGRIND_START_INSTRUMENTATION;
  CALLGRIND_TOGGLE_COLLECT;
}
void m6_callgrind_stop(void) {
  CALLGRIND_TOGGLE_COLLECT;
  CALLGRIND_STOP_INSTRUMENTATION;
}
