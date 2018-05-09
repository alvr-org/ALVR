/*
 * Module: sched.h
 *
 * Purpose:
 *      Provides an implementation of POSIX realtime extensions
 *      as defined in
 *
 *              POSIX 1003.1b-1993      (POSIX.1b)
 *
 * --------------------------------------------------------------------------
 *
 *      Pthreads-win32 - POSIX Threads Library for Win32
 *      Copyright(C) 1998 John E. Bossom
 *      Copyright(C) 1999,2012 Pthreads-win32 contributors
 *
 *      Homepage1: http://sourceware.org/pthreads-win32/
 *      Homepage2: http://sourceforge.net/projects/pthreads4w/
 *
 *      The current list of contributors is contained
 *      in the file CONTRIBUTORS included with the source
 *      code distribution. The list can also be seen at the
 *      following World Wide Web location:
 *      http://sources.redhat.com/pthreads-win32/contributors.html
 *
 *      This library is free software; you can redistribute it and/or
 *      modify it under the terms of the GNU Lesser General Public
 *      License as published by the Free Software Foundation; either
 *      version 2 of the License, or (at your option) any later version.
 *
 *      This library is distributed in the hope that it will be useful,
 *      but WITHOUT ANY WARRANTY; without even the implied warranty of
 *      MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *      Lesser General Public License for more details.
 *
 *      You should have received a copy of the GNU Lesser General Public
 *      License along with this library in the file COPYING.LIB;
 *      if not, write to the Free Software Foundation, Inc.,
 *      59 Temple Place - Suite 330, Boston, MA 02111-1307, USA
 */
#if !defined(_SCHED_H)
#define _SCHED_H

#if defined(_MSC_VER)
#  if _MSC_VER < 1300
#    define PTW32_CONFIG_MSVC6
#  endif
#  if _MSC_VER < 1400
#    define PTW32_CONFIG_MSVC7
#  endif
#endif

#undef PTW32_SCHED_LEVEL

#if defined(_POSIX_SOURCE)
#define PTW32_SCHED_LEVEL 0
/* Early POSIX */
#endif

#if defined(_POSIX_C_SOURCE) && _POSIX_C_SOURCE >= 199309
#undef PTW32_SCHED_LEVEL
#define PTW32_SCHED_LEVEL 1
/* Include 1b, 1c and 1d */
#endif

#if defined(INCLUDE_NP)
#undef PTW32_SCHED_LEVEL
#define PTW32_SCHED_LEVEL 2
/* Include Non-Portable extensions */
#endif

#define PTW32_SCHED_LEVEL_MAX 3

#if ( defined(_POSIX_C_SOURCE) && _POSIX_C_SOURCE >= 200112 )  || !defined(PTW32_SCHED_LEVEL)
#define PTW32_SCHED_LEVEL PTW32_SCHED_LEVEL_MAX
/* Include everything */
#endif


#if defined(__GNUC__) && !defined(__declspec)
# error Please upgrade your GNU compiler to one that supports __declspec.
#endif

/*
 * When building the library, you should define PTW32_BUILD so that
 * the variables/functions are exported correctly. When using the library,
 * do NOT define PTW32_BUILD, and then the variables/functions will
 * be imported correctly.
 */
#if !defined(PTW32_STATIC_LIB)
#  if defined(PTW32_BUILD)
#    define PTW32_DLLPORT __declspec (dllexport)
#  else
#    define PTW32_DLLPORT __declspec (dllimport)
#  endif
#else
#  define PTW32_DLLPORT
#endif

/*
 * The Open Watcom C/C++ compiler uses a non-standard calling convention
 * that passes function args in registers unless __cdecl is explicitly specified
 * in exposed function prototypes.
 *
 * We force all calls to cdecl even though this could slow Watcom code down
 * slightly. If you know that the Watcom compiler will be used to build both
 * the DLL and application, then you can probably define this as a null string.
 * Remember that sched.h (this file) is used for both the DLL and application builds.
 */
#if !defined(PTW32_CDECL)
# define PTW32_CDECL __cdecl
#endif

/*
 * This is a duplicate of what is in the autoconf config.h,
 * which is only used when building the pthread-win32 libraries.
 */

#if !defined(PTW32_CONFIG_H)
#  if defined(WINCE)
#    define NEED_ERRNO
#    define NEED_SEM
#  endif
#  if defined(__MINGW64__)
#    define HAVE_STRUCT_TIMESPEC
#    define HAVE_MODE_T
#  elif defined(_UWIN) || defined(__MINGW32__)
#    define HAVE_MODE_T
#  endif
#endif

/*
 *
 */

#include <stdlib.h>

#if PTW32_SCHED_LEVEL >= PTW32_SCHED_LEVEL_MAX
#if defined(NEED_ERRNO)
#include "need_errno.h"
#else
#include <errno.h>
#endif
#endif /* PTW32_SCHED_LEVEL >= PTW32_SCHED_LEVEL_MAX */

#if (defined(__MINGW64__) || defined(__MINGW32__)) || defined(_UWIN)
# if PTW32_SCHED_LEVEL >= PTW32_SCHED_LEVEL_MAX
/* For pid_t */
#  include <sys/types.h>
/* Required by Unix 98 */
#  include <time.h>
# else
   typedef int pid_t;
# endif
#else
 /* [i_a] fix for using pthread_win32 with mongoose code, which #define's its own pid_t akin to     typedef HANDLE pid_t; */
 #undef pid_t
# if defined(_MSC_VER)
  typedef void *pid_t;
# else
  typedef int pid_t;
# endif
#endif

/*
 * Microsoft VC++6.0 lacks these *_PTR types
 */
#if defined(_MSC_VER) && _MSC_VER < 1300 && !defined(PTW32_HAVE_DWORD_PTR)
typedef unsigned long ULONG_PTR;
typedef ULONG_PTR DWORD_PTR;
#endif

/* Thread scheduling policies */

enum {
  SCHED_OTHER = 0,
  SCHED_FIFO,
  SCHED_RR,
  SCHED_MIN   = SCHED_OTHER,
  SCHED_MAX   = SCHED_RR
};

struct sched_param {
  int sched_priority;
};

/*
 * CPU affinity
 *
 * cpu_set_t:
 * Considered opaque but cannot be an opaque pointer
 * due to the need for compatibility with GNU systems
 * and sched_setaffinity() et.al. which include the
 * cpusetsize parameter "normally set to sizeof(cpu_set_t)".
 */

#define CPU_SETSIZE (sizeof(size_t)*8)

#define CPU_COUNT(setptr) (_sched_affinitycpucount(setptr))

#define CPU_ZERO(setptr) (_sched_affinitycpuzero(setptr))

#define CPU_SET(cpu, setptr) (_sched_affinitycpuset((cpu),(setptr)))

#define CPU_CLR(cpu, setptr) (_sched_affinitycpuclr((cpu),(setptr)))

#define CPU_ISSET(cpu, setptr) (_sched_affinitycpuisset((cpu),(setptr)))

#define CPU_AND(destsetptr, srcset1ptr, srcset2ptr) (_sched_affinitycpuand((destsetptr),(srcset1ptr),(srcset2ptr)))

#define CPU_OR(destsetptr, srcset1ptr, srcset2ptr) (_sched_affinitycpuor((destsetptr),(srcset1ptr),(srcset2ptr)))

#define CPU_XOR(destsetptr, srcset1ptr, srcset2ptr) (_sched_affinitycpuxor((destsetptr),(srcset1ptr),(srcset2ptr)))

#define CPU_EQUAL(set1ptr, set2ptr) (_sched_affinitycpuequal((set1ptr),(set2ptr)))

typedef union
{
  char cpuset[CPU_SETSIZE/8];
  size_t _align;
} cpu_set_t;

#if defined(__cplusplus)
extern "C"
{
#endif                          /* __cplusplus */

PTW32_DLLPORT int PTW32_CDECL sched_yield (void);

PTW32_DLLPORT int PTW32_CDECL sched_get_priority_min (int policy);

PTW32_DLLPORT int PTW32_CDECL sched_get_priority_max (int policy);

PTW32_DLLPORT int PTW32_CDECL sched_setscheduler (pid_t pid, int policy);

PTW32_DLLPORT int PTW32_CDECL sched_getscheduler (pid_t pid);

/* Compatibility with Linux - not standard */

PTW32_DLLPORT int PTW32_CDECL sched_setaffinity (pid_t pid, size_t cpusetsize, cpu_set_t *mask);

PTW32_DLLPORT int PTW32_CDECL sched_getaffinity (pid_t pid, size_t cpusetsize, cpu_set_t *mask);

/*
 * Support routines and macros for cpu_set_t
 */
PTW32_DLLPORT int PTW32_CDECL _sched_affinitycpucount (const cpu_set_t *set);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuzero (cpu_set_t *pset);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuset (int cpu, cpu_set_t *pset);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuclr (int cpu, cpu_set_t *pset);

PTW32_DLLPORT int PTW32_CDECL _sched_affinitycpuisset (int cpu, const cpu_set_t *pset);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuand(cpu_set_t *pdestset, const cpu_set_t *psrcset1, const cpu_set_t *psrcset2);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuor(cpu_set_t *pdestset, const cpu_set_t *psrcset1, const cpu_set_t *psrcset2);

PTW32_DLLPORT void PTW32_CDECL _sched_affinitycpuxor(cpu_set_t *pdestset, const cpu_set_t *psrcset1, const cpu_set_t *psrcset2);

PTW32_DLLPORT int PTW32_CDECL _sched_affinitycpuequal (const cpu_set_t *pset1, const cpu_set_t *pset2);

/*
 * Note that this macro returns ENOTSUP rather than
 * ENOSYS as might be expected. However, returning ENOSYS
 * should mean that sched_get_priority_{min,max} are
 * not implemented as well as sched_rr_get_interval.
 * This is not the case, since we just don't support
 * round-robin scheduling. Therefore I have chosen to
 * return the same value as sched_setscheduler when
 * SCHED_RR is passed to it.
 */
#define sched_rr_get_interval(_pid, _interval) \
  ( errno = ENOTSUP, (int) -1 )


#if defined(__cplusplus)
}                               /* End of extern "C" */
#endif                          /* __cplusplus */

#undef PTW32_SCHED_LEVEL
#undef PTW32_SCHED_LEVEL_MAX

#endif                          /* !_SCHED_H */

