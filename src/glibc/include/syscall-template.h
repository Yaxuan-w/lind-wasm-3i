#ifndef _SYSCALL_TEMPLATE_H
#define _SYSCALL_TEMPLATE_H

#include <errno.h>
#include <sysdep.h>
#include <sys/syscall.h>

/* Template for system calls */
#define SYSCALL_TEMPLATE(name, nr, type, parameters...) \
type name parameters                                    \
{                                                      \
    long int resultvar;                                \
    INTERNAL_SYSCALL_DECL(err);                        \
    resultvar = INTERNAL_SYSCALL(nr, err, parameters); \
    if (INTERNAL_SYSCALL_ERROR_P(resultvar, err))      \
    {                                                  \
        __set_errno(INTERNAL_SYSCALL_ERRNO(resultvar, err)); \
        return (type)-1;                               \
    }                                                  \
    return (type)resultvar;                           \
}

/* Template for void system calls */
#define SYSCALL_TEMPLATE_VOID(name, nr, parameters...)  \
void name parameters                                    \
{                                                      \
    INTERNAL_SYSCALL_DECL(err);                        \
    INTERNAL_SYSCALL(nr, err, parameters);             \
}

#endif /* _SYSCALL_TEMPLATE_H */
