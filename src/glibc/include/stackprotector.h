/* Details about stack protection enablement and how to disable it.
   Copyright (C) 2022 Free Software Foundation, Inc.
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

#ifndef _INCLUDE_STACKPROTECTOR_H
#define _INCLUDE_STACKPROTECTOR_H

#include <config.h>

/* Used to disable stack protection in sensitive places, like ifunc
   resolvers and early static TLS init.  */
#ifdef HAVE_CC_NO_STACK_PROTECTOR
# ifdef __clang__
#  define inhibit_stack_protector \
     __attribute__((no_stack_protector))
# else
#  define inhibit_stack_protector \
    __attribute__ ((__optimize__ ("-fno-stack-protector")))
# endif
#else
# define inhibit_stack_protector
#endif

#endif
