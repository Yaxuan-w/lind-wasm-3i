/* Copyright (C) 2015-2024 Free Software Foundation, Inc.
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
   License along with the GNU C Library; if not, see
   <https://www.gnu.org/licenses/>.  */

#include <sys/socket.h>
#include <socketcall.h>
#include <syscall-template.h>
#include <lind_syscall_num.h>

int
__socket (int fd, int type, int domain)
{
// #ifdef __ASSUME_SOCKET_SYSCALL
//   return INLINE_SYSCALL_CALL (socket, fd, type, domain);
// #else
//   return SOCKETCALL (socket, fd, type, domain);
// #endif
  // Dennis Edit
  return MAKE_SYSCALL(SOCKET_SYSCALL, "syscall|socket", (uint64_t) fd, (uint64_t) type, (uint64_t) domain, NOTUSED, NOTUSED, NOTUSED);
}
libc_hidden_def (__socket)
weak_alias (__socket, socket)
