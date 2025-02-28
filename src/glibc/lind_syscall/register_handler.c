#include <stdint.h> // For uint64_t definition
#include <syscall-template.h> // For make_syscall definition

int register_handler(uint64_t targetcage, uint64_t targetcallnum, uint64_t handlefunc, uint64_t handlefunccage) {
    return MAKE_SYSCALL(400, "register_handler", targetcage, targetcallnum, handlefunc, handlefunccage, NOTUSED, NOTUSED);
}
