/* Create create pipe.  Linux generic version.
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
#include <unistd.h>
#include <sysdep.h>
#include <syscall-template.h>
#include <lind_syscall_num.h>
/* Create a one-way communication channel (__pipe).
   If successful, two file descriptors are stored in PIPEDES;
   bytes written on PIPEDES[1] can be read from PIPEDES[0].
   Returns 0 if successful, -1 if not.  */
int
__pipe (int pipedes[2])
{
   return MAKE_SYSCALL(PIPE_SYSCALL, "syscall|pipe", (uint64_t) pipedes, NOTUSED, NOTUSED, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__pipe)
weak_alias (__pipe, pipe)
