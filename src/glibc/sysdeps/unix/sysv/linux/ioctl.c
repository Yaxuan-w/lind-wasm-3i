/* Control device.  Linux generic implementation.
   Copyright (C) 2021-2024 Free Software Foundation, Inc.
   This file is part of the GNU C Library.

   The GNU C Library is free software; you can redistribute it and/or
   modify it under the terms of the GNU Lesser General Public
   License as published by the Free Software Foundation; either
   version 2.1 of the License, or (at your option) any later version.

   The GNU C Library is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
   Lesser General Public License for more details.

   You should have received a copy of the GNU Lesser General Public
   License along with the GNU C Library.  If not, see
   <https://www.gnu.org/licenses/>.  */

#include <stdarg.h>
#include <sys/ioctl.h>
#include <sysdep.h>
#include <internal-ioctl.h>
#include <syscall-template.h>
#include <lind_syscall_num.h>
int
__ioctl (int fd, unsigned long int request, ...)
{
  va_list args;
  va_start (args, request);
  void *arg = va_arg (args, void *);
  va_end (args);
        return MAKE_SYSCALL(IOCTL_SYSCALL, "syscall|ioctl", (uint64_t) fd, (uint64_t) request, (uint64_t) arg, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__ioctl)
weak_alias (__ioctl, ioctl)
#if __TIMESIZE != 64
strong_alias (__ioctl, __ioctl_time64)
#endif
