/* Linux open syscall implementation, non-LFS.
   Copyright (C) 2017-2024 Free Software Foundation, Inc.
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

#include <fcntl.h>
#include <stdarg.h>
#include <unistd.h>
#include <sysdep.h>
#include <syscall-template.h>
#include <lind_syscall_num.h>
#ifndef __OFF_T_MATCHES_OFF64_T

/* Open FILE with access OFLAG.  If O_CREAT or O_TMPFILE is in OFLAG,
   a third argument is the file protection.  */
int
__open (const char *file, int oflag, ...)
{
  mode_t mode = 0;
  if (oflag & O_CREAT)
    {
      va_list arg;
      va_start (arg, oflag);
      mode = va_arg (arg, mode_t);
      va_end (arg);
    }

  return MAKE_SYSCALL(OPEN_SYSCALL, "syscall|open", (uint64_t) file, (uint64_t) oflag, (uint64_t) mode, NOTUSED, NOTUSED, NOTUSED);
}

int open (const char *file, int oflag, ...)
{
  mode_t mode = 0;
  if (oflag & O_CREAT)
    {
      va_list arg;
      va_start (arg, oflag);
      mode = va_arg (arg, mode_t);
      va_end (arg);
    }

  return MAKE_SYSCALL(OPEN_SYSCALL, "syscall|open", (uint64_t) file, (uint64_t) oflag, (uint64_t) mode, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__open)
weak_alias (__open, open)
libc_hidden_def (open)

