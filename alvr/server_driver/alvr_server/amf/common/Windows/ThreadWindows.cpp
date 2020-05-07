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
// Copyright (c) 2016 Advanced Micro Devices, Inc. All rights reserved.
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
#include <timeapi.h>
#include <windows.h>
//----------------------------------------------------------------------------------------
// threading
//----------------------------------------------------------------------------------------
amf_long AMF_CDECL_CALL amf_atomic_inc(amf_long* X)
{
    return InterlockedIncrement((long*)X);
}
//----------------------------------------------------------------------------------------
amf_long AMF_CDECL_CALL amf_atomic_dec(amf_long* X)
{
    return InterlockedDecrement((long*)X);
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_create_critical_section()
{
    CRITICAL_SECTION* cs = new CRITICAL_SECTION;
#if defined(METRO_APP)
    ::InitializeCriticalSectionEx(cs, 0, CRITICAL_SECTION_NO_DEBUG_INFO);
#else
    ::InitializeCriticalSection(cs);
#endif
    return (amf_handle)cs; // in Win32 - no errors
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_delete_critical_section(amf_handle cs)
{
    ::DeleteCriticalSection((CRITICAL_SECTION*)cs);
    delete (CRITICAL_SECTION*)cs;
    return true; // in Win32 - no errors
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_enter_critical_section(amf_handle cs)
{
    ::EnterCriticalSection((CRITICAL_SECTION*)cs);
    return true; // in Win32 - no errors
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_leave_critical_section(amf_handle cs)
{
    ::LeaveCriticalSection((CRITICAL_SECTION*)cs);
    return true; // in Win32 - no errors
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_create_event(bool bInitiallyOwned, bool bManualReset, const wchar_t* pName)
{
#if defined(METRO_APP)
    DWORD flags = ((bManualReset) ? CREATE_EVENT_MANUAL_RESET : 0) |
        ((bInitiallyOwned) ? CREATE_EVENT_INITIAL_SET : 0);

    return ::CreateEventEx(NULL, pName, flags, STANDARD_RIGHTS_ALL | EVENT_MODIFY_STATE);

#else
    return ::CreateEventW(NULL, bManualReset == true, bInitiallyOwned == true, pName);

#endif
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_delete_event(amf_handle hevent)
{
    return ::CloseHandle(hevent) != FALSE;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_set_event(amf_handle hevent)
{
    return ::SetEvent(hevent) != FALSE;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_reset_event(amf_handle hevent)
{
    return ::ResetEvent(hevent) != FALSE;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_wait_for_event(amf_handle hevent, amf_ulong ulTimeout)
{
#if defined(METRO_APP)
    return ::WaitForSingleObjectEx(hevent, ulTimeout, FALSE) == WAIT_OBJECT_0;

#else
    return ::WaitForSingleObject(hevent, ulTimeout) == WAIT_OBJECT_0;

#endif
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_wait_for_event_timeout(amf_handle hevent, amf_ulong ulTimeout)
{
    DWORD ret;
#if defined(METRO_APP)
    ret = ::WaitForSingleObjectEx(hevent, ulTimeout, FALSE);
#else
    ret = ::WaitForSingleObject(hevent, ulTimeout);
#endif
    return ret == WAIT_OBJECT_0 || ret == WAIT_TIMEOUT;
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_create_mutex(bool bInitiallyOwned, const wchar_t* pName)
{
#if defined(METRO_APP)
    DWORD flags = (bInitiallyOwned) ? CREATE_MUTEX_INITIAL_OWNER : 0;
    return ::CreateMutexEx(NULL, pName, flags, STANDARD_RIGHTS_ALL);

#else
    return ::CreateMutexW(NULL, bInitiallyOwned == true, pName);

#endif
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_open_mutex(const wchar_t* pName)
{
    return ::OpenMutexW(MUTEX_ALL_ACCESS, FALSE, pName);
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_delete_mutex(amf_handle hmutex)
{
    return ::CloseHandle(hmutex) != FALSE;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_wait_for_mutex(amf_handle hmutex, amf_ulong ulTimeout)
{
#if defined(METRO_APP)
    return ::WaitForSingleObjectEx(hmutex, ulTimeout, FALSE) == WAIT_OBJECT_0;

#else
    return ::WaitForSingleObject(hmutex, ulTimeout) == WAIT_OBJECT_0;

#endif
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_release_mutex(amf_handle hmutex)
{
    return ::ReleaseMutex(hmutex) != FALSE;
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_create_semaphore(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName)
{
    if(iMaxCount == NULL)
    {
        return NULL;
    }
#if defined(METRO_APP)
    return ::CreateSemaphoreEx(NULL, iInitCount, iMaxCount, pName, 0, STANDARD_RIGHTS_ALL | SEMAPHORE_MODIFY_STATE);

#else
    return ::CreateSemaphoreW(NULL, iInitCount, iMaxCount, pName);

#endif
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_delete_semaphore(amf_handle hsemaphore)
{
    if(hsemaphore == NULL)
    {
        return true;
    }
    return ::CloseHandle(hsemaphore) != FALSE;
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_wait_for_semaphore(amf_handle hsemaphore, amf_ulong timeout)
{
    if(hsemaphore == NULL)
    {
        return true;
    }
#if defined(METRO_APP)
    return ::WaitForSingleObjectEx(hsemaphore, timeout, false) == WAIT_OBJECT_0;

#else
    return ::WaitForSingleObject(hsemaphore, timeout) == WAIT_OBJECT_0;

#endif
}
//----------------------------------------------------------------------------------------
bool AMF_CDECL_CALL amf_release_semaphore(amf_handle hsemaphore, amf_long iCount, amf_long* iOldCount)
{
    if(hsemaphore == NULL)
    {
        return true;
    }
    return ::ReleaseSemaphore(hsemaphore, iCount, iOldCount) != FALSE;
}
//------------------------------------------------------------------------------
void AMF_CDECL_CALL amf_sleep(amf_ulong delay)
{
#if defined(METRO_APP)
    Concurrency::wait(delay);
#else
    Sleep(delay);
#endif
}
//----------------------------------------------------------------------------------------
amf_pts AMF_CDECL_CALL amf_high_precision_clock()
{
    static int state = 0;
    static LARGE_INTEGER Frequency;
    static LARGE_INTEGER StartCount;
    if(state == 0)
    {
        if(QueryPerformanceFrequency(&Frequency))
        {
            state = 1;
            QueryPerformanceCounter(&StartCount);
        }
        else
        {
            state = 2;
        }
    }
    if(state == 1)
    {
        LARGE_INTEGER PerformanceCount;
        if(QueryPerformanceCounter(&PerformanceCount))
        {
            return static_cast<amf_pts>((PerformanceCount.QuadPart - StartCount.QuadPart) * 10000000LL / Frequency.QuadPart);
        }
    }
#if defined(METRO_APP)
    return GetTickCount64() * 10;

#else
    return GetTickCount() * 10;

#endif
}
//-------------------------------------------------------------------------------------------------
#pragma comment (lib, "Winmm.lib")
static amf_uint32 timerPrecision = 1;

void AMF_CDECL_CALL amf_increase_timer_precision()
{
#if !defined(METRO_APP)
    while (timeBeginPeriod(timerPrecision) == TIMERR_NOCANDO)
    {
        ++timerPrecision;
    }
/*
    typedef NTSTATUS (CALLBACK * NTSETTIMERRESOLUTION)(IN ULONG DesiredTime,IN BOOLEAN SetResolution,OUT PULONG ActualTime);
    typedef NTSTATUS (CALLBACK * NTQUERYTIMERRESOLUTION)(OUT PULONG MaximumTime,OUT PULONG MinimumTime,OUT PULONG CurrentTime);

    HINSTANCE hNtDll = LoadLibrary(L"NTDLL.dll");
    if(hNtDll != NULL)
    {
        ULONG MinimumResolution=0;
        ULONG MaximumResolution=0;
        ULONG ActualResolution=0;

        NTQUERYTIMERRESOLUTION NtQueryTimerResolution = (NTQUERYTIMERRESOLUTION)GetProcAddress(hNtDll, "NtQueryTimerResolution");
        NTSETTIMERRESOLUTION NtSetTimerResolution = (NTSETTIMERRESOLUTION)GetProcAddress(hNtDll, "NtSetTimerResolution");

        if(NtQueryTimerResolution != NULL && NtSetTimerResolution != NULL)
        {
            NtQueryTimerResolution (&MinimumResolution, &MaximumResolution, &ActualResolution);
            if(MaximumResolution != 0)
            {
                NtSetTimerResolution (MaximumResolution, TRUE, &ActualResolution);
                NtQueryTimerResolution (&MinimumResolution, &MaximumResolution, &ActualResolution);

                // if call NtQueryTimerResolution() again it will return the same values but precision is actually increased
            }
        }
        FreeLibrary(hNtDll);
    }
*/
#endif
}
void AMF_CDECL_CALL amf_restore_timer_precision()
{
#if !defined(METRO_APP)
    timeEndPeriod(timerPrecision);
#endif
}
//----------------------------------------------------------------------------------------
amf_handle AMF_CDECL_CALL amf_load_library(const wchar_t* filename)
{
#if defined(METRO_APP)
    return LoadPackagedLibrary(filename, 0);
#else
    return ::LoadLibraryW(filename);
#endif
}
//----------------------------------------------------------------------------------------
void* AMF_CDECL_CALL amf_get_proc_address(amf_handle module, const char* procName)
{
    return ::GetProcAddress((HMODULE)module, procName);
}
//----------------------------------------------------------------------------------------
int AMF_CDECL_CALL amf_free_library(amf_handle module)
{
    return ::FreeLibrary((HMODULE)module)==TRUE;
}
#if !defined(METRO_APP)
//----------------------------------------------------------------------------------------
// memory
//----------------------------------------------------------------------------------------
void* AMF_CDECL_CALL amf_virtual_alloc(size_t size)
{
    return VirtualAlloc(NULL, size, MEM_COMMIT, PAGE_READWRITE);
}
//----------------------------------------------------------------------------------------
void AMF_CDECL_CALL amf_virtual_free(void* ptr)
{
    VirtualFree(ptr, NULL, MEM_RELEASE);
}
#endif //#if !defined(METRO_APP)//----------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------
