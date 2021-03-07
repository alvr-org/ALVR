// 
// Notice Regarding Standards.  AMD does not provide a license or sublicense to
// any Intellectual Property Rights relating to any standards, including but not
// limited to any audio and/or video codec technologies such as MPEG-2, MPEG-4;
// AVC/H.264; HEVC/H.265; AAC decode/FFMPEG; AAC encode/FFMPEG; VC-1; and MP3
// (collectively, the "Media Technologies"). For clarity, you will pay any
// royalties due for such third party technologies, which may include the Media
// Technologies that are owed as a result of AMD providing the Software to you.
// 
// MIT license 
// 
// Copyright (c) 2018 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//


#include "../Thread.h"


#if defined (__linux) || (__APPLE__)

#if defined(__GNUC__)
    //disable gcc warinings on STL code
    #pragma GCC diagnostic ignored "-Weffc++"
#endif

#define POSIX

#include <locale>
#include <algorithm>
#include <dirent.h>
#include <fnmatch.h>
#include <pwd.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdlib.h>
#include <errno.h>
#include <time.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <dlfcn.h>
#include <sys/time.h>	

#if !defined(__APPLE__)
#include <malloc.h>
#endif

#if defined(__ANDROID__)
#include <android/log.h>
#endif

#include <sys/types.h>
#include <semaphore.h>
#include <pthread.h>

#include "../AMFSTL.h"

using namespace amf;

extern "C" void AMF_STD_CALL amf_debug_trace(const wchar_t* text);


void perror(const char* errorModule)
{
    char buf[128];
#if defined(__ANDROID__) || (__APPLE__)
    strerror_r(errno, buf, sizeof(buf));
    fprintf(stderr, "%s: %s", buf, errorModule);
#else
    char* err = strerror_r(errno, buf, sizeof(buf));
    fprintf(stderr, "%s: %s", err, errorModule);
#endif

    exit(1);
}

#if defined(__APPLE__)
amf_uint64 AMF_STD_CALL get_current_thread_id()
{
    return reinterpret_cast<amf_uint64>(pthread_self());
}
#else
amf_uint32 AMF_STD_CALL get_current_thread_id()
{
    return static_cast<amf_uint32>(pthread_self());
}
#endif


// int clock_gettime(clockid_t clk_id, struct timespec *tp);
//----------------------------------------------------------------------------------------
// threading
//----------------------------------------------------------------------------------------
amf_long AMF_STD_CALL amf_atomic_inc(amf_long* X)
{
    return __sync_add_and_fetch(X, 1);
}
//----------------------------------------------------------------------------------------
amf_long AMF_STD_CALL amf_atomic_dec(amf_long* X)
{
    return __sync_sub_and_fetch(X, 1);
}
//----------------------------------------------------------------------------------------
amf_handle AMF_STD_CALL amf_create_critical_section()
{
    pthread_mutex_t* mutex = new pthread_mutex_t;
    pthread_mutexattr_t attr;
    pthread_mutexattr_init(&attr);
    pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_RECURSIVE);
    pthread_mutex_init(mutex, &attr);

    return (amf_handle)mutex;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_delete_critical_section(amf_handle cs)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)cs;
    int err = pthread_mutex_destroy(mutex);
    delete mutex;
    return err == 0;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_enter_critical_section(amf_handle cs)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)cs;
    return pthread_mutex_lock(mutex) == 0;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_wait_critical_section(amf_handle cs, amf_ulong ulTimeout)
{
    return amf_wait_for_mutex(cs, ulTimeout);
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_leave_critical_section(amf_handle cs)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)cs;
    return pthread_mutex_unlock(mutex) == 0;
}
//----------------------------------------------------------------------------------------
struct MyEvent
{
    bool m_manual_reset;
    pthread_cond_t m_cond;
    pthread_mutex_t m_mutex;
    bool m_triggered;
};
//----------------------------------------------------------------------------------------

amf_handle AMF_STD_CALL amf_create_event(bool initially_owned, bool manual_reset, const wchar_t* name)
{
    MyEvent* event = new MyEvent;


    // Linux does not natively support Named Condition variables
    // so raise an error.
    // Implement this using boost (NamedCondition), Qt, or some other framework.
    if(name != NULL)
    {
        perror("Named Events not supported under Linux yet");
        exit(1);
    }
    event->m_manual_reset = manual_reset;
    pthread_cond_t cond_tmp = PTHREAD_COND_INITIALIZER;
    event->m_cond = cond_tmp;
    pthread_mutex_t mutex_tmp = PTHREAD_MUTEX_INITIALIZER;
    event->m_mutex = mutex_tmp;

    event->m_triggered = false;
    if(initially_owned)
    {
        amf_set_event((amf_handle)event);
    }

    return (amf_handle)event;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_delete_event(amf_handle hevent)
{
    MyEvent* event = (MyEvent*)hevent;
    int err1 = pthread_mutex_destroy(&event->m_mutex);
    int err2 = pthread_cond_destroy(&event->m_cond);
    delete event;
    return err1 == 0 && err2 == 0;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_set_event(amf_handle hevent)
{
    MyEvent* event = (MyEvent*)hevent;
    pthread_mutex_lock(&event->m_mutex);
    event->m_triggered = true;
    int err1 = pthread_cond_broadcast(&event->m_cond);
    pthread_mutex_unlock(&event->m_mutex);

    return err1 == 0;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_reset_event(amf_handle hevent)
{
    MyEvent* event = (MyEvent*)hevent;
    pthread_mutex_lock(&event->m_mutex);
    event->m_triggered = false;
    int err = pthread_mutex_unlock(&event->m_mutex);

    return err == 0;
}
//----------------------------------------------------------------------------------------
static bool AMF_STD_CALL amf_wait_for_event_int(amf_handle hevent, unsigned long timeout, bool bTimeoutErr)
{
    bool ret = true;
    int err = 0;
    MyEvent* event = (MyEvent*)hevent;
    pthread_mutex_lock(&event->m_mutex);

    timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    amf_uint64 start_time = ((amf_uint64)ts.tv_sec) * 1000 + ((amf_uint64)ts.tv_nsec) / 1000000; //to msec

    if(event->m_manual_reset)
    {
        while(!event->m_triggered)
        {
            if(timeout == AMF_INFINITE)
            {
                err = pthread_cond_wait(&event->m_cond, &event->m_mutex);          //MM todo - timeout is not supported
                ret = err == 0;
            }
            else
            {
                clock_gettime(CLOCK_REALTIME, &ts);
                amf_uint64 current_time = ((amf_uint64)ts.tv_sec) * 1000 + ((amf_uint64)ts.tv_nsec) / 1000000; //to msec
                if(current_time - start_time > (amf_uint64)timeout)
                {
                    ret = bTimeoutErr ? false : true;
                    break;
                }
                amf_uint64 to_wait = start_time + timeout;

                timespec abstime;
                abstime.tv_sec = (time_t)(to_wait / 1000); // timeout is in millisec
                abstime.tv_nsec = (time_t)((to_wait - ((amf_uint64)abstime.tv_sec) * 1000) * 1000000); // the rest to nanosec

                err = pthread_cond_timedwait(&event->m_cond, &event->m_mutex, &abstime);
                ret = err == 0;
            }
        }
    }
    else
    {
        if(event->m_triggered)
        {
            ret = true;
        }
        else
        {
            if (timeout == AMF_INFINITE) {
                err = pthread_cond_wait(&event->m_cond, &event->m_mutex);
            } else {
                start_time += timeout;
                timespec abstime;
                abstime.tv_sec = (time_t) (start_time / 1000); // timeout is in millisec
                abstime.tv_nsec = (time_t) ((start_time - (amf_uint64) (abstime.tv_sec) * 1000) *
                                            1000000); // the rest to nanosec
                err = pthread_cond_timedwait(&event->m_cond, &event->m_mutex, &abstime);
            }

            if (bTimeoutErr) {
                ret = (err == 0);
            } else {
                ret = (err == 0 || err == ETIMEDOUT);
            }
        }
        if(ret == true)
        {
            event->m_triggered = false;
        }
    }
    pthread_mutex_unlock(&event->m_mutex);

    return ret;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_wait_for_event(amf_handle hevent, unsigned long timeout)
{
    return amf_wait_for_event_int(hevent, timeout, true);
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_wait_for_event_timeout(amf_handle hevent, amf_ulong ulTimeout)
{
    return amf_wait_for_event_int(hevent, ulTimeout, false);
}
//----------------------------------------------------------------------------------------
amf_handle AMF_STD_CALL amf_create_mutex(bool initially_owned, const wchar_t* name)
{
    pthread_mutex_t* mutex = new pthread_mutex_t;
    pthread_mutex_t mutex_tmp = PTHREAD_MUTEX_INITIALIZER;
    *mutex = mutex_tmp;

    if(initially_owned)
    {
        pthread_mutex_lock(mutex);
    }
    return (amf_handle)mutex;
}
//----------------------------------------------------------------------------------------
amf_handle AMF_STD_CALL amf_open_mutex(const wchar_t* pName)
{
    assert(false);
    return 0;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_delete_mutex(amf_handle hmutex)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)hmutex;
    int err = pthread_mutex_destroy(mutex);
    delete mutex;
    return err == 0;
}
//----------------------------------------------------------------------------------------

#if defined(__APPLE__)
int sem_timedwait1(sem_t* semaphore, const struct timespec* timeout)
{
    struct timeval timenow;
    struct timespec sleepytime;
    int retcode;
    
    /// This is just to avoid a completely busy wait
    sleepytime.tv_sec = 0;
    sleepytime.tv_nsec = 10000000; // 10ms

    while((retcode = sem_trywait(semaphore)) != 0)
    {
        gettimeofday (&timenow, NULL);
        
        if((timenow.tv_sec >= timeout->tv_sec) && ((timenow.tv_usec * 1000) >= timeout->tv_nsec))
        {
            return retcode;
        }
        nanosleep (&sleepytime, NULL);
    }
    return retcode;
}
#endif

#if defined(__ANDROID__)
int pthread_mutex_timedlock1(pthread_mutex_t* mutex, const struct timespec* timeout)
{
    struct timeval timenow;
    struct timespec sleepytime;
    int retcode;

    /// This is just to avoid a completely busy wait
    sleepytime.tv_sec = 0;
    sleepytime.tv_nsec = 10000000; // 10ms

    while((retcode = pthread_mutex_trylock (mutex)) == EBUSY)
    {
        gettimeofday (&timenow, NULL);

        if((timenow.tv_sec >= timeout->tv_sec) && ((timenow.tv_usec * 1000) >= timeout->tv_nsec))
        {
            return ETIMEDOUT;
        }
        nanosleep (&sleepytime, NULL);
    }
    return retcode;
}
#endif

//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_wait_for_mutex(amf_handle hmutex, unsigned long timeout)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)hmutex;
    if(timeout == AMF_INFINITE)
    {
        return pthread_mutex_lock(mutex) == 0;
    }

    // ulTimeout is in milliseconds
    long timeout_sec = timeout / 1000;      /* Seconds */;
    long timeout_nsec = (timeout - (timeout / 1000) * 1000) * 1000000;

    timespec wait_time; //absolute time
    clock_gettime(CLOCK_REALTIME, &wait_time);

    wait_time.tv_sec += timeout_sec;
    wait_time.tv_nsec += timeout_nsec;

    if (wait_time.tv_nsec >= 1000000000)
    {
        wait_time.tv_sec++;
        wait_time.tv_nsec -= 1000000000;
    }

#ifdef __APPLE__
    int* tmpptr = NULL;
    *tmpptr = 1;
//    assert(false); // not supported
    return false;

#else
#if defined(__ANDROID__)
    return pthread_mutex_timedlock1(mutex, &wait_time) == 0;
#else
    return pthread_mutex_timedlock(mutex, &wait_time) == 0;
#endif
#endif
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_release_mutex(amf_handle hmutex)
{
    pthread_mutex_t* mutex = (pthread_mutex_t*)hmutex;
    return pthread_mutex_unlock(mutex) != 0;
}

//----------------------------------------------------------------------------------------
amf_handle AMF_STD_CALL amf_create_semaphore(amf_long iInitCount, amf_long iMaxCount, const wchar_t* /*pName*/)
{
    if(iMaxCount == 0 || iInitCount > iMaxCount)
    {
        return NULL;
    }

    sem_t* semaphore = new sem_t;
    if(sem_init(semaphore, 0, iInitCount) != 0)
    {
        delete semaphore;
        return NULL;
    }
    return (amf_handle)semaphore;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_delete_semaphore(amf_handle hsemaphore)
{
    bool ret = true;
    if(hsemaphore == NULL)
    {
        return true;
    }
    sem_t* semaphore = (sem_t*)hsemaphore;
    ret = (0==sem_destroy(semaphore)) ? 1:0;
    delete semaphore;
    return ret;
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_wait_for_semaphore(amf_handle hsemaphore, amf_ulong timeout)
{
    if(hsemaphore == NULL)
    {
        return true;
    }

    // ulTimeout is in milliseconds
    long timeout_sec = timeout / 1000;      /* Seconds */;
    long timeout_nsec = (timeout - (timeout / 1000) * 1000) * 1000000;

    timespec wait_time; //absolute time
    clock_gettime(CLOCK_REALTIME, &wait_time);

    wait_time.tv_sec += timeout_sec;
    wait_time.tv_nsec += timeout_nsec;

    if (wait_time.tv_nsec >= 1000000000)
    {
        wait_time.tv_sec++;
        wait_time.tv_nsec -= 1000000000;
    }

    sem_t* semaphore = (sem_t*)hsemaphore;
    if(timeout != AMF_INFINITE)
    {
    #if defined(__APPLE__)
        return sem_timedwait1 (semaphore, &wait_time) == 0; // errno=ETIMEDOU
    #else
        return sem_timedwait (semaphore, &wait_time) == 0; // errno=ETIMEDOUT
    #endif
    }
    else
    {
        return sem_wait(semaphore) == 0;
    }
}
//----------------------------------------------------------------------------------------
bool AMF_STD_CALL amf_release_semaphore(amf_handle hsemaphore, amf_long iCount, amf_long* iOldCount)
{
    if(hsemaphore == NULL)
    {
        return true;
    }
    sem_t* semaphore = (sem_t*)hsemaphore;

    if(iOldCount != NULL)
    {
        int iTmp = 0;
        sem_getvalue(semaphore, &iTmp);
        *iOldCount = iTmp;
    }

    for(int i = 0; i < iCount; i++)
    {
        sem_post(semaphore);
    }
    return true;
}
//------------------------------------------------------------------------------
/*
 * Delay is specified in milliseconds.
 * Function will return prematurely if msDelay value is invalid.
 *
 * */
void AMF_STD_CALL amf_sleep(amf_ulong msDelay)
{
#if defined(NANOSLEEP_DONTUSE)
    struct timespec sts, sts_remaining;
    int iErrorCode;

    ts.tv_sec = msDelay / 1000;
    ts.tv_nsec = (msDelay - sts.tv_sec * 1000) * 1000000; // nanosec
    // put in code to measure sleep clock jitter
    do
    {
        iErrorCode = nanosleep(&sts, &sts_remaining);
        if(iErrorCode)
        {
            switch(errno)
            {
            case EINTR:
                sts = sts_remaining;
                break;

            case EFAULT:
            case EINVAL:
            case default:
                perror("amf_sleep");
                return;
                /* TODO: how to log errors? */
            }
        }
    } while(iErrorCode);
#else
    usleep(msDelay * 1000);
#endif
}

//----------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------
// memory
//----------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------
void AMF_STD_CALL amf_debug_trace(const wchar_t* text)
{
#if defined(__ANDROID__)
    __android_log_write(ANDROID_LOG_DEBUG, "AMF_TRACE", amf_from_unicode_to_multibyte(text).c_str());
#else
    fprintf(stderr, "%ls", text);
#endif
}

void* AMF_STD_CALL amf_virtual_alloc(size_t size)
{
    void* mem = NULL;
#if defined(__ANDROID__)
    mem = memalign(sysconf(_SC_PAGESIZE), size);
    if(mem == NULL)
    {
        amf_debug_trace(L"Failed to alloc memory using memalign() function.");
    }
#else
    int exitCode = posix_memalign(&mem, sysconf(_SC_PAGESIZE), size);
    if(exitCode != 0)
    {
        amf_debug_trace(L"Failed to alloc memory using posix_memaling() function.");
    }
#endif

    return mem;
}
//-------------------------------------------------------------------------------------------------------
void AMF_STD_CALL amf_virtual_free(void* ptr)
{
    free(ptr); // according to linux help memory allocated by memalign() must be freed by free()
}
//----------------------------------------------------------------------------------------
void* AMF_STD_CALL amf_aligned_alloc(size_t count, size_t alignment)
{
#if defined(__APPLE__)
    void* p = nullptr;
    posix_memalign(&p, alignment, count);
    return p;
#else
    return memalign(alignment, count);
#endif
}
//----------------------------------------------------------------------------------------
void AMF_STD_CALL amf_aligned_free(void* ptr)
{
    return free(ptr);
}

amf_handle AMF_STD_CALL amf_load_library(const wchar_t* filename)
{
    void *ret = dlopen(amf_from_unicode_to_multibyte(filename).c_str(), RTLD_NOW | RTLD_GLOBAL);
    if(ret ==0 )
    {
        const char *err = dlerror();
        int a=1;
    }
    return ret;
}

void* AMF_STD_CALL amf_get_proc_address(amf_handle module, const char* procName)
{
    return dlsym(module, procName);
}
//-------------------------------------------------------------------------------------------------
int AMF_STD_CALL amf_free_library(amf_handle module)
{
    return dlclose(module) == 0;
}
void AMF_STD_CALL amf_increase_timer_precision()
{
}
void AMF_STD_CALL amf_restore_timer_precision()
{
}
//----------------------------------------------------------------------------------------
double AMF_STD_CALL amf_clock()
{
    //MM: clock() Win32 - returns time from beginning of the program
    //MM: clock() works different in Linux - returns consumed processor time
    timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    double cur_time = ((double)ts.tv_sec) + ((double)ts.tv_nsec) / 1000000000.; //to sec
    return cur_time;
}
//----------------------------------------------------------------------------------------
amf_int64 AMF_STD_CALL get_time_in_seconds_with_fraction()
{
   struct timeval tv;

   gettimeofday(&tv, NULL);

   amf_int64 ntp_time = ((tv.tv_sec * 1000) + (tv.tv_usec / 1000));
    return ntp_time;
}
//---------------------------------------------------------------------------------------
amf_pts AMF_STD_CALL amf_high_precision_clock()
{
    timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return ts.tv_sec * 10000000LL + ts.tv_nsec / 100.; //to nanosec
}
//--------------------------------------------------------------------------------
// the end
//--------------------------------------------------------------------------------

#endif
