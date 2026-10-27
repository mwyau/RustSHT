#include <valgrind/callgrind.h>
void callgrind_profile_start(void) { CALLGRIND_START_INSTRUMENTATION; }
void callgrind_profile_stop(void) { CALLGRIND_STOP_INSTRUMENTATION; }
