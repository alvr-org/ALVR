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

#if defined(_WIN32)
#include <process.h>
#else
#include <pthread.h>
#endif
#include "Thread.h"

#if defined(METRO_APP)
    #include <ppl.h>
    #include <ppltasks.h>
#endif



namespace amf
{
    //----------------------------------------------------------------------------
    AMFEvent::AMFEvent(bool bInitiallyOwned, bool bManualReset, const wchar_t* pName) : m_hSyncObject()
    {
        m_hSyncObject = amf_create_event(bInitiallyOwned, bManualReset, pName);
    }
    //----------------------------------------------------------------------------
    AMFEvent::~AMFEvent()
    {
        amf_delete_event(m_hSyncObject);
    }
    //----------------------------------------------------------------------------
    bool AMFEvent::Lock(amf_ulong ulTimeout)
    {
        return amf_wait_for_event(m_hSyncObject, ulTimeout);
    }
    //----------------------------------------------------------------------------
    bool AMFEvent::LockTimeout(amf_ulong ulTimeout)
    {
        return amf_wait_for_event_timeout(m_hSyncObject, ulTimeout);
    }
    //----------------------------------------------------------------------------
    bool AMFEvent::Unlock()
    {
        return true;
    }
    //----------------------------------------------------------------------------
    bool AMFEvent::SetEvent()
    {
        return amf_set_event(m_hSyncObject);
    }
    //----------------------------------------------------------------------------
    bool AMFEvent::ResetEvent()
    {
        return amf_reset_event(m_hSyncObject);
    }
    //----------------------------------------------------------------------------
    //----------------------------------------------------------------------------
    AMFMutex::AMFMutex(bool bInitiallyOwned, const wchar_t* pName
                   #if defined(_WIN32)
                       , bool bOpenExistent
                   #endif
                       ):m_hSyncObject()
    {
#if defined(_WIN32)
        if(bOpenExistent)
        {
            m_hSyncObject = amf_open_mutex(pName);
        }
        else
#else
    //#pragma message AMF_TODO("Open mutex!!! missing functionality in Linux!!!")
#endif
        {
            m_hSyncObject = amf_create_mutex(bInitiallyOwned, pName);
        }
    }
    //----------------------------------------------------------------------------
    AMFMutex::~AMFMutex()
    {
        if(m_hSyncObject)
        {
            amf_delete_mutex(m_hSyncObject);
        }
    }
    //----------------------------------------------------------------------------
    bool AMFMutex::Lock(amf_ulong ulTimeout)
    {
        if(m_hSyncObject)
        {
            return amf_wait_for_mutex(m_hSyncObject, ulTimeout);
        }
        else
        {
            return false;
        }
    }
    //----------------------------------------------------------------------------
    bool AMFMutex::Unlock()
    {
        if(m_hSyncObject)
        {
            return amf_release_mutex(m_hSyncObject);
        }
        else
        {
            return false;
        }
    }
    //----------------------------------------------------------------------------
    bool AMFMutex::IsValid()
    {
        return m_hSyncObject != NULL;
    }
    //----------------------------------------------------------------------------
    //----------------------------------------------------------------------------
    AMFCriticalSection::AMFCriticalSection() : m_Sect()
    {
        m_Sect = amf_create_critical_section();
    }
    //----------------------------------------------------------------------------
    AMFCriticalSection::~AMFCriticalSection()
    {
        amf_delete_critical_section(m_Sect);
    }
    //----------------------------------------------------------------------------
    bool AMFCriticalSection::Lock(amf_ulong ulTimeout)
    {
        (void)ulTimeout;
        return amf_enter_critical_section(m_Sect);
    }
    //----------------------------------------------------------------------------
    bool AMFCriticalSection::Unlock()
    {
        return amf_leave_critical_section(m_Sect);
    }
    //----------------------------------------------------------------------------
    AMFSemaphore::AMFSemaphore(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName)
        : m_hSemaphore(NULL)
    {
        Create(iInitCount, iMaxCount, pName);
    }
    //----------------------------------------------------------------------------
    AMFSemaphore::~AMFSemaphore()
    {
        amf_delete_semaphore(m_hSemaphore);
    }
    //----------------------------------------------------------------------------
    bool AMFSemaphore::Create(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName)
    {
        if(m_hSemaphore != NULL)  // delete old one
        {
            amf_delete_semaphore(m_hSemaphore);
            m_hSemaphore = NULL;
        }
        if(iMaxCount > 0)
        {
            m_hSemaphore = amf_create_semaphore(iInitCount, iMaxCount, pName);
        }
        return true;
    }
    //----------------------------------------------------------------------------
    bool AMFSemaphore::Lock(amf_ulong ulTimeout)
    {
        return amf_wait_for_semaphore(m_hSemaphore, ulTimeout);
    }
    //----------------------------------------------------------------------------
    bool AMFSemaphore::Unlock()
    {
        amf_long iOldCount = 0;
        return amf_release_semaphore(m_hSemaphore, 1, &iOldCount);
    }
    //----------------------------------------------------------------------------

    //----------------------------------------------------------------------------
    //----------------------------------------------------------------------------
    AMFLock::AMFLock(AMFSyncBase* pBase, amf_ulong ulTimeout)
        : m_pBase(pBase),
        m_bLocked()
    {
        m_bLocked = Lock(ulTimeout);
    }
    //----------------------------------------------------------------------------
    AMFLock::~AMFLock()
    {
        Unlock();
    }
    //----------------------------------------------------------------------------
    bool AMFLock::Lock(amf_ulong ulTimeout)
    {
        if(m_pBase == NULL)
        {
            return false;
        }
        m_bLocked = m_pBase->Lock(ulTimeout);
        return m_bLocked;
    }
    //----------------------------------------------------------------------------
    bool AMFLock::Unlock()
    {
        if(m_pBase == NULL)
        {
            return false;
        }
        m_bLocked = m_pBase->Unlock();
        return m_bLocked;
    }
    //----------------------------------------------------------------------------
    bool AMFLock::IsLocked()
    {
        return m_bLocked;
    }
    //----------------------------------------------------------------------------

    #if defined(METRO_APP)
    using namespace Platform;
    using namespace Windows::Foundation;
    using namespace Windows::UI::Xaml;
    using namespace Windows::UI::Xaml::Controls;
    using namespace Windows::UI::Xaml::Navigation;
        class AMFThreadObj
        {
            Windows::Foundation::IAsyncAction^      m_AsyncAction;
            AMFEvent                                m_StopEvent;
            AMFThread*                              m_pOwner;
        public:
            AMFThreadObj(AMFThread* owner);
            virtual ~AMFThreadObj();

            virtual bool Start();
            virtual bool RequestStop();
            virtual bool WaitForStop();
            virtual bool StopRequested();

            // this is executed in the thread and overloaded by implementor
            virtual void Run() { m_pOwner->Run(); }
            virtual bool Init(){ return m_pOwner->Init(); }
            virtual bool Terminate(){ return m_pOwner->Terminate();}
        };


    AMFThreadObj::AMFThreadObj(AMFThread* owner)
        : m_StopEvent(true, true), m_pOwner(owner)
    {}

    AMFThreadObj::~AMFThreadObj()
    {}

    bool AMFThreadObj::Start()
    {
        auto workItemDelegate = [this](IAsyncAction ^ workItem)
        {
            if( !this->Init() )
            {
                return;
            }

            this->Run();
            this->Terminate();

            this->m_AsyncAction = nullptr;
            if( this->StopRequested() )
            {
                this->m_StopEvent.SetEvent();
            }

        };

        Windows::System::Threading::WorkItemPriority WorkPriority;
        WorkPriority = Windows::System::Threading::WorkItemPriority::Normal;

        auto workItemHandler = ref new Windows::System::Threading::WorkItemHandler(workItemDelegate);
        m_AsyncAction = Windows::System::Threading::ThreadPool::RunAsync(workItemHandler, WorkPriority);

        return true;
    }

    bool AMFThreadObj::RequestStop()
    {
        if( m_AsyncAction == nullptr )
        {
            return true;
        }

        m_StopEvent.ResetEvent();
        return true;
    }

    bool AMFThreadObj::WaitForStop()
    {
        if( m_AsyncAction == nullptr )
        {
            return true;
        }

        return m_StopEvent.Lock();
    }

    bool AMFThreadObj::StopRequested()
    {
        return !m_StopEvent.Lock(0);
    }
    bool AMFThreadObj::IsRunning()
    {
        return m_AsyncAction != nullptr;
    }

    void amf::ExitThread()
    {}

    //#endif//#if defined(METRO_APP)
    //#if defined(_WIN32)
    #elif defined(_WIN32)   // _WIN32 and METRO_APP defines are not mutually exclusive
    class AMFThreadObj
    {
        AMFThread*      m_pOwner;
        uintptr_t       m_pThread;
        AMFEvent        m_StopEvent;
    public:
        // this icalled by owner
        AMFThreadObj(AMFThread* owner);
        virtual ~AMFThreadObj();

        virtual bool Start();
        virtual bool RequestStop();
        virtual bool WaitForStop();
        virtual bool StopRequested();
        virtual bool IsRunning();


    protected:
        static void AMF_CDECL_CALL AMFThreadProc(void* pThis);

        // this is executed in the thread and overloaded by implementor
        virtual void Run()
        {
            m_pOwner->Run();
        }
        virtual bool Init()
        {
            return m_pOwner->Init();
        }
        virtual bool Terminate()
        {
            return m_pOwner->Terminate();
        }
    };
    //----------------------------------------------------------------------------
    AMFThreadObj::AMFThreadObj(AMFThread* owner)
        : m_pThread(uintptr_t(-1)),
        m_StopEvent(true, true), m_pOwner(owner)
    {}
    //----------------------------------------------------------------------------
    AMFThreadObj::~AMFThreadObj()
    {
        //    RequestStop();
        //    WaitForStop();
    }
    //----------------------------------------------------------------------------
    void AMF_CDECL_CALL AMFThreadObj::AMFThreadProc(void* pThis)
    {
        AMFThreadObj* pT = (AMFThreadObj*)pThis;
        if(!pT->Init())
        {
            return;
        }
        pT->Run();
        pT->Terminate();

        pT->m_pThread = uintptr_t(-1);
        if(pT->StopRequested())
        {
            pT->m_StopEvent.SetEvent(); // signal to stop that we just finished
        }
    }
    //----------------------------------------------------------------------------
    bool AMFThreadObj::Start()
    {
        if(m_pThread != (uintptr_t)-1L)
        {
            return true;
        }

        m_pThread = _beginthread(AMFThreadProc, 0, (void* )this);

        return m_pThread != (uintptr_t)-1L;
    }

    //----------------------------------------------------------------------------
    bool AMFThreadObj::RequestStop()
    {
        if(m_pThread == (uintptr_t)-1L)
        {
            return true;
        }

        m_StopEvent.ResetEvent();
        return true;
    }
    //----------------------------------------------------------------------------
    bool AMFThreadObj::WaitForStop()
    {
        if(m_pThread == (uintptr_t)-1L)
        {
            return true;
        }
        return m_StopEvent.Lock();
    }
    //----------------------------------------------------------------------------
    bool AMFThreadObj::StopRequested()
    {
        return !m_StopEvent.Lock(0);
    }
    bool AMFThreadObj::IsRunning()
    {
        return m_pThread != (uintptr_t)-1L;
    }
    //----------------------------------------------------------------------------
    void amf::ExitThread()
    {
        _endthread();
    }

    #endif //#if defined(_WIN32)
    #if defined(__linux)
        class AMFThreadObj
        {
        public:
            AMFThreadObj(AMFThread* owner);
            virtual ~AMFThreadObj();

            virtual bool Start();
            virtual bool RequestStop();
            virtual bool WaitForStop();
            virtual bool StopRequested();

            // this is executed in the thread and overloaded by implementor
            virtual void Run() { m_pOwner->Run(); }
            virtual bool Init(){ return m_pOwner->Init(); }
            virtual bool Terminate(){ return m_pOwner->Terminate();}

        private:
            AMFThread*      m_pOwner;
            pthread_t m_hThread;
            bool m_bStopRequested;
            pthread_mutex_t m_hMutex;

            AMFThreadObj(const AMFThreadObj&);
            AMFThreadObj& operator=(const AMFThreadObj&);
        };

    AMFThreadObj::AMFThreadObj(AMFThread* owner)
        : m_pOwner(owner),
        m_hThread(0),
        m_bStopRequested(false),
        m_hMutex()
    {
        pthread_mutex_init(&m_hMutex, 0);
    }

    AMFThreadObj::~AMFThreadObj()
    {
        pthread_mutex_destroy(&m_hMutex);
    }

    void* AMF_CDECL_CALL AMFThreadProc(void* pThis)
    {
        AMFThreadObj* pT = (AMFThreadObj*)pThis;
        if(!pT->Init())
        {
            return 0;
        }
        pT->Run();
        pT->Terminate();
        return 0;
    }

    bool AMFThreadObj::Start()
    {
        return 0 == pthread_create(&m_hThread, 0, AMFThreadProc, (void*)this);
    }

    bool AMFThreadObj::RequestStop()
    {
        pthread_mutex_lock(&m_hMutex);
        m_bStopRequested = true;
        pthread_mutex_unlock(&m_hMutex);
        return true;
    }

    bool AMFThreadObj::WaitForStop()
    {
        if(m_hThread)
        {
            pthread_join(m_hThread, 0);
        }
        return true;
    }

    bool AMFThreadObj::StopRequested()
    {
        pthread_mutex_lock(&m_hMutex);
        bool bRet = m_bStopRequested;
        pthread_mutex_unlock(&m_hMutex);
        return bRet;
    }

    void ExitThread()
    {
        pthread_exit(0);
    }

    #endif //#if defined(__linux)

    AMFThread::AMFThread() : m_thread()
    {
        m_thread = new AMFThreadObj(this);
    }

    AMFThread::~AMFThread()
    {
        delete m_thread;
    }

    bool AMFThread::Start()
    {
        return m_thread->Start();
    }

    bool AMFThread::RequestStop()
    {
        return m_thread->RequestStop();
    }

    bool AMFThread::WaitForStop()
    {
        return m_thread->WaitForStop();
    }

    bool AMFThread::StopRequested()
    {
        return m_thread->StopRequested();
    }
    bool AMFThread::IsRunning()
    {
        return m_thread->IsRunning();
    }
} //namespace
