/* Benchmark utility functions.
   Copyright (C) 2015-2024 Free Software Foundation, Inc.
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

/* Prevent compiler to optimize away call.  */
#define DO_NOT_OPTIMIZE_OUT(value)		  \
  ({						  \
    __typeof (value) __v = (value);		  \
    asm volatile ("" : : "r,m" (__v) : "memory"); \
    __v;					  \
  })

#if __GNUC_PREREQ (4, 4) || __glibc_has_attribute (__optimize__)
# define attribute_optimize(level) __attribute__ ((optimize (level)))
#else
# define attribute_optimize(level)
#endif

#ifndef START_ITER
# define START_ITER (100000000)
#endif

/* bench_start reduces the random variations due to frequency scaling by
   executing a small loop with many memory accesses.  START_ITER controls
   the number of iterations.  */

void bench_start (void);
