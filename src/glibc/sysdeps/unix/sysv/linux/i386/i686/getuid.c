#include <unistd.h>
#include <syscall-template.h>

__uid_t
__getuid (void)
{
  return MAKE_SYSCALL(50, "syscall|getuid", (uint64_t) file, (uint64_t) oflag, (uint64_t) mode, NOTUSED, NOTUSED, NOTUSED);
}

__uid_t
getuid (void)
{
  return MAKE_SYSCALL(50, "syscall|getuid", (uint64_t) file, (uint64_t) oflag, (uint64_t) mode, NOTUSED, NOTUSED, NOTUSED);
}

