/* Duplicate a file descriptor.  Linux version.
   Copyright (C) 2011-2024 Free Software Foundation, Inc.
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
#include <unistd.h>
#include <sysdep.h>
#include <syscall-template.h>

int
__dup2 (int fd, int fd2)
{
   return MAKE_SYSCALL(25, "syscall|dup2", (uint64_t) fd, (uint64_t) fd2, NOTUSED, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__dup2)
weak_alias (__dup2, dup2)