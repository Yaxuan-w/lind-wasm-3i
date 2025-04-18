/* Create a directory.  Linux version.
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
#include <sys/stat.h>
#include <sysdep.h>
#include <syscall-template.h>
#include <lind_syscall_num.h>

/* Create a directory named PATH with protections MODE.  */
int
__mkdir (const char *path, mode_t mode)
{
   return MAKE_SYSCALL(MKDIR_SYSCALL, "syscall|mkdir", (uint64_t) path, (uint64_t) mode, NOTUSED, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__mkdir)
weak_alias (__mkdir, mkdir)
