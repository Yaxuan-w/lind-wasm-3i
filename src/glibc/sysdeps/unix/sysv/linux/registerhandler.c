#include <unistd.h>
#include <sys/uio.h>
#include <syscall-template.h>

ssize_t
__registerhandler (int targetcageid, int targetcallnum, void *handlefunc, int handlefunccage)
{
  return MAKE_SYSCALL(400, "syscall|registerhandler", (uint64_t) targetcageid, (uint64_t) targetcallnum, (uint64_t)(uintptr_t) handlefunc, (uint64_t) handlefunccage, NOTUSED, NOTUSED);
}

strong_alias (__registerhandler, registerhandler)
