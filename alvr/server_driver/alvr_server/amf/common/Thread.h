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

#ifndef __AMFThread_h__
#define __AMFThread_h__
#pragma once

#include <cassert>
#include <list>
#include <vector>

#include "../include/core/Platform.h"

extern "C"
{
    // threads
    #define AMF_INFINITE        (0xFFFFFFFF) // Infinite ulTimeout

    // threads: atomic
    amf_long    AMF_CDECL_CALL amf_atomic_inc(amf_long* X);
    amf_long    AMF_CDECL_CALL amf_atomic_dec(amf_long* X);

    // threads: critical section
    amf_handle  AMF_CDECL_CALL amf_create_critical_section();
    bool        AMF_CDECL_CALL amf_delete_critical_section(amf_handle cs);
    bool        AMF_CDECL_CALL amf_enter_critical_section(amf_handle cs);
    bool        AMF_CDECL_CALL amf_leave_critical_section(amf_handle cs);
    // threads: event
    amf_handle  AMF_CDECL_CALL amf_create_event(bool bInitiallyOwned, bool bManualReset, const wchar_t* pName);
    bool        AMF_CDECL_CALL amf_delete_event(amf_handle hevent);
    bool        AMF_CDECL_CALL amf_set_event(amf_handle hevent);
    bool        AMF_CDECL_CALL amf_reset_event(amf_handle hevent);
    bool        AMF_CDECL_CALL amf_wait_for_event(amf_handle hevent, amf_ulong ulTimeout);
    bool        AMF_CDECL_CALL amf_wait_for_event_timeout(amf_handle hevent, amf_ulong ulTimeout);

    // threads: mutex
    amf_handle  AMF_CDECL_CALL amf_create_mutex(bool bInitiallyOwned, const wchar_t* pName);
#if defined(_WIN32)
    amf_handle  AMF_CDECL_CALL amf_open_mutex(const wchar_t* pName);
#endif
    bool        AMF_CDECL_CALL amf_delete_mutex(amf_handle hmutex);
    bool        AMF_CDECL_CALL amf_wait_for_mutex(amf_handle hmutex, amf_ulong ulTimeout);
    bool        AMF_CDECL_CALL amf_release_mutex(amf_handle hmutex);

    // threads: semaphore
    amf_handle  AMF_CDECL_CALL amf_create_semaphore(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName);
    bool        AMF_CDECL_CALL amf_delete_semaphore(amf_handle hsemaphore);
    bool        AMF_CDECL_CALL amf_wait_for_semaphore(amf_handle hsemaphore, amf_ulong ulTimeout);
    bool        AMF_CDECL_CALL amf_release_semaphore(amf_handle hsemaphore, amf_long iCount, amf_long* iOldCount);

    // threads: delay
    void        AMF_CDECL_CALL amf_sleep(amf_ulong delay);
    amf_pts     AMF_CDECL_CALL amf_high_precision_clock();    // in 100 of nanosec

    void        AMF_CDECL_CALL amf_increase_timer_precision();
    void        AMF_CDECL_CALL amf_restore_timer_precision();

    amf_handle  AMF_CDECL_CALL amf_load_library(const wchar_t* filename);
    void*       AMF_CDECL_CALL amf_get_proc_address(amf_handle module, const char* procName);
    int         AMF_CDECL_CALL amf_free_library(amf_handle module);

#if !defined(METRO_APP)
    // virtual memory
    void*       AMF_CDECL_CALL amf_virtual_alloc(amf_size size);
    void        AMF_CDECL_CALL amf_virtual_free(void* ptr);
#else
    #define amf_virtual_alloc amf_alloc
    #define amf_virtual_free amf_free
#endif


}

namespace amf
{
    //----------------------------------------------------------------
    class AMF_NO_VTABLE AMFSyncBase
    {
    public:
        virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE) = 0;
        virtual bool Unlock() = 0;
    };
    //----------------------------------------------------------------
    class AMFEvent : public AMFSyncBase
    {
    private:
        amf_handle m_hSyncObject;

        AMFEvent(const AMFEvent&);
        AMFEvent& operator=(const AMFEvent&);

    public:
        AMFEvent(bool bInitiallyOwned = false, bool bManualReset = false, const wchar_t* pName = NULL);
        virtual ~AMFEvent();

        virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE);
        virtual bool LockTimeout(amf_ulong ulTimeout = AMF_INFINITE);
        virtual bool Unlock();
        bool SetEvent();
        bool ResetEvent();
    };
    //----------------------------------------------------------------
    class AMFMutex : public AMFSyncBase
    {
    private:
        amf_handle m_hSyncObject;

        AMFMutex(const AMFMutex&);
        AMFMutex& operator=(const AMFMutex&);

    public:
        AMFMutex(bool bInitiallyOwned = false, const wchar_t* pName = NULL
        #if defined(_WIN32)
            , bool bOpenExistent = false
        #endif
            );
        virtual ~AMFMutex();

        virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE);
        virtual bool Unlock();
        bool IsValid();
    };
    //----------------------------------------------------------------
    class AMFCriticalSection : public AMFSyncBase
    {
    private:
        amf_handle m_Sect;

        AMFCriticalSection(const AMFCriticalSection&);
        AMFCriticalSection& operator=(const AMFCriticalSection&);

    public:
        AMFCriticalSection();
        virtual ~AMFCriticalSection();

        virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE);
        virtual bool Unlock();
    };
    //----------------------------------------------------------------
    class AMFSemaphore : public AMFSyncBase
    {
    private:
        amf_handle m_hSemaphore;

        AMFSemaphore(const AMFSemaphore&);
        AMFSemaphore& operator=(const AMFSemaphore&);

    public:
        AMFSemaphore(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName = NULL);
        virtual ~AMFSemaphore();

        virtual bool Create(amf_long iInitCount, amf_long iMaxCount, const wchar_t* pName = NULL);
        virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE);
        virtual bool Unlock();
    };
    //----------------------------------------------------------------
    class AMFLock
    {
    private:
        AMFSyncBase* m_pBase;
        bool m_bLocked;

        AMFLock(const AMFLock&);
        AMFLock& operator=(const AMFLock&);

    public:
        AMFLock(AMFSyncBase* pBase, amf_ulong ulTimeout = AMF_INFINITE);
        ~AMFLock();

        bool Lock(amf_ulong ulTimeout = AMF_INFINITE);
        bool Unlock();
        bool IsLocked();
    };
    //----------------------------------------------------------------
    class AMFReadWriteSync
    {
    private:
        struct ReadWriteResources
        {
            // max threads reading concurrently
            const int m_maxReadThreads;
            AMFSemaphore m_readSemaphore;
            AMFCriticalSection m_writeCriticalSection;
            ReadWriteResources() : 
            m_maxReadThreads(10),
                m_readSemaphore(m_maxReadThreads, m_maxReadThreads),
                m_writeCriticalSection()
            { }
        };
        class ReadSync : public AMFSyncBase
        {
        private:
            ReadSync(const ReadSync&);
            ReadSync& operator=(const ReadSync&);

            ReadWriteResources& m_resources;
        public:
            ReadSync(ReadWriteResources& resources) : m_resources(resources)
            { }
            virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE)
            {
                return m_resources.m_readSemaphore.Lock(ulTimeout);
            }
            virtual bool Unlock()
            {
                return m_resources.m_readSemaphore.Unlock();
            }
        };
        class WriteSync : public AMFSyncBase
        {
        private:
            WriteSync(const WriteSync&);
            WriteSync& operator=(const WriteSync&);

            ReadWriteResources& m_resources;
        public:
            WriteSync(ReadWriteResources& resources) : m_resources(resources)
            { }
            /// waits passed timeout for other writers; wait readers for infinite
            virtual bool Lock(amf_ulong ulTimeout = AMF_INFINITE)
            {
                if(!m_resources.m_writeCriticalSection.Lock(ulTimeout))
                {
                    return false;
                }
                for(int i = 0; i < m_resources.m_maxReadThreads; i++)
                {
                    m_resources.m_readSemaphore.Lock();
                }
                return true;
            }
            virtual bool Unlock()
            {
                // there is windows function to release N times by one call - could be optimize later
                for(int i = 0; i < m_resources.m_maxReadThreads; i++)
                {
                    m_resources.m_readSemaphore.Unlock();
                }
                return m_resources.m_writeCriticalSection.Unlock();
            }
        };
    private:
        ReadWriteResources m_resources;
        ReadSync m_readSync;
        WriteSync m_writeSync;
    public:
        AMFReadWriteSync() :
            m_resources(),
            m_readSync(m_resources),
            m_writeSync(m_resources)
        { }

        AMFSyncBase* GetReadSync()
        {
            return &m_readSync;
        }
        AMFSyncBase* GetWriteSync()
        {
            return &m_writeSync;
        }
    };
    //----------------------------------------------------------------
    class AMFThreadObj;
    class AMFThread
    {
    public:
        AMFThread();
        virtual ~AMFThread();

        virtual bool Start();
        virtual bool RequestStop();
        virtual bool WaitForStop();
        virtual bool StopRequested();
        virtual bool IsRunning();

        // this is executed in the thread and overloaded by implementor
        virtual void Run() = 0;
        virtual bool Init()
        {
            return true;
        }
        virtual bool Terminate()
        {
            return true;
        }
    private:
        AMFThreadObj* m_thread;

        AMFThread(const AMFThread&);
        AMFThread& operator=(const AMFThread&);
    };

    void ExitThread();
    //----------------------------------------------------------------
    template<typename T>
    class AMFQueue
    {
    protected:
        class ItemData
        {
        public:
            T data;
            amf_ulong ulID;
            amf_long ulPriority;
            ItemData() : data(), ulID(), ulPriority(){}
        };
        typedef std::list< ItemData > QueueList;

        QueueList m_Queue;
        AMFCriticalSection m_cSect;
        AMFEvent m_SomethingInQueueEvent;
        AMFSemaphore m_QueueSizeSem;
        amf_int32 m_iQueueSize;

        bool InternalGet(amf_ulong& ulID, T& item)
        {
            AMFLock lock(&m_cSect);
            if(!m_Queue.empty())  // something to get
            {
                ItemData& itemdata = m_Queue.front();
                ulID = itemdata.ulID;
                item = itemdata.data;
                m_Queue.pop_front();
                m_QueueSizeSem.Unlock();
                if(m_Queue.empty())
                {
                    m_SomethingInQueueEvent.ResetEvent();
                }
                return true;
            }
            return false;
        }
    public:
        AMFQueue(amf_int32 iQueueSize = 0)
            : m_Queue(),
            m_cSect(),
            m_SomethingInQueueEvent(false, false),
            m_QueueSizeSem(iQueueSize, iQueueSize > 0 ? iQueueSize + 1 : 0),
            m_iQueueSize(iQueueSize) {}
        virtual ~AMFQueue(){}

        virtual bool SetQueueSize(amf_int32 iQueueSize)
        {
            bool success = m_QueueSizeSem.Create(iQueueSize, iQueueSize > 0 ? iQueueSize + 1 : 0);
            if(success)
            {
                m_iQueueSize = iQueueSize;
            }
            return success;
        }
        virtual amf_int32 GetQueueSize()
        {
            return m_iQueueSize;
        }
        virtual bool Add(amf_ulong ulID, const T& item, amf_long ulPriority = 0, amf_ulong ulTimeout = AMF_INFINITE)
        {
            if(m_QueueSizeSem.Lock(ulTimeout) == false)
            {
                return false;
            }
            {
                AMFLock lock(&m_cSect);


                ItemData itemdata;
                itemdata.ulID = ulID;
                itemdata.data = item;
                itemdata.ulPriority = ulPriority;

                typename QueueList::iterator iter = m_Queue.end();

                for(; iter != m_Queue.begin(); )
                {
                    iter--;
                    if(ulPriority <= (iter->ulPriority))
                    {
                        iter++;
                        break;
                    }
                }
                m_Queue.insert(iter, itemdata);
                m_SomethingInQueueEvent.SetEvent(); // this will set all waiting threads - some of them get data, some of them not
            }
            return true;
        }

        virtual bool Get(amf_ulong& ulID, T& item, amf_ulong ulTimeout)
        {
            if(InternalGet(ulID, item))  // try right away
            {
                return true;
            }
            // wait for queue
            if(m_SomethingInQueueEvent.Lock(ulTimeout))
            {
                return InternalGet(ulID, item);
            }
            return false;
        }
        virtual void Clear()
        {
            bool bValue = true;
            while(bValue)
            {
                amf_ulong ulID;
                T item;
                bValue = InternalGet(ulID, item);
            }
        }
        virtual amf_size GetSize()
        {
            AMFLock lock(&m_cSect);
            return m_Queue.size();
        }
    };
    //----------------------------------------------------------------
    template<class inT, class outT>
    class AMFQueueThread : public AMFThread
    {
    private:
        AMFQueueThread(const AMFQueueThread&);
        AMFQueueThread& operator=(const AMFQueueThread&);

    protected:
        AMFQueue<inT>* m_pInQueue;
        AMFQueue<outT>* m_pOutQueue;
        AMFMutex m_mutexInProcess;  ///< This mutex shows other threads that the thread function allocates
        ///< some objects on stack and it is unsafe state. To manipulate objects owned by descendant classes
        ///< client must lock this mutex by calling BlockProcessing member function. When client finished its work
        ///< corresponding UnblockProcessing member function call must be done.

        bool m_blockProcessingRequested;
        AMFCriticalSection m_csBlockingRequest;
    public:
        AMFQueueThread(AMFQueue<inT>* pInQueue,
            AMFQueue<outT>* pOutQueue) : m_pInQueue(pInQueue), m_pOutQueue(pOutQueue), m_mutexInProcess(),
            m_blockProcessingRequested(false), m_csBlockingRequest()
        {}
        virtual bool Process(amf_ulong& ulID, inT& inData, outT& outData) = 0;
        virtual void BlockProcessing()
        {
            AMFLock lock(&m_csBlockingRequest);
            m_blockProcessingRequested = true;
            m_mutexInProcess.Lock();
        }
        virtual void UnblockProcessing()
        {
            AMFLock lock(&m_csBlockingRequest);
            m_mutexInProcess.Unlock();
            m_blockProcessingRequested = false;
        }
        virtual bool IsPaused()
        {
            return false;
        }
        virtual void OnHaveOutput() {}
        virtual void OnIdle() {}

        virtual void Run()
        {
            bool bStop = false;
            while(!bStop)
            {
                {
                    AMFLock lock(&m_mutexInProcess);
                    inT inData;
                    amf_ulong ulID = 0;
                    bool callProcess = true;
                    if(m_pInQueue != NULL)
                    {
                        amf_ulong waitTimeout = 5;
                        bool validInput = m_pInQueue->Get(ulID, inData, waitTimeout); // Pulse to check Stop from time to time
                        if(StopRequested())
                        {
                            bStop = true;
                        }
                        if(!validInput)
                        {
                            callProcess = false;
                        }
                    }
                    if(!bStop && callProcess)
                    {
                        outT outData;
                        bool validOutput = Process(ulID, inData, outData);
                        if(StopRequested())
                        {
                            bStop = true;
                        }
                        if(!bStop  && (m_pOutQueue != NULL) && validOutput)
                        {
                            m_pOutQueue->Add(ulID, outData);
                            OnHaveOutput();
                        }
                    }
                    else
                    {
                        OnIdle();
                    }
                }
                if(StopRequested())
                {
                    bStop = true;
                }
#if defined(__linux)
                ///< HACK
                ///< This amf_sleep(0) is required to emulate windows mutex behavior.
                ///< In Windows release mutext causes some other waiting thread is receiving ownership of mutex.
                ///< In Linux it is not true.
                ///< Without sleep AMFLock destructor releases mutex but immediately on next cycle AMFLock constructor tries to lock
                ///< the mutex and now system have two threads waiting for the mutex.
                ///< Using some random logic system decides who will be unlocked.
                ///< This thread may win during several seconds it looks like pipeline is hang.
                ///< amf_sleep call causes waiting thread is becoming unlocked.
                if(m_blockProcessingRequested)
                {
                    amf_sleep(0);
                }
#endif
            }
        }
    };
    //----------------------------------------------------------------
    template<class inT, class outT, class _Thread, class ThreadParam>
    class AMFQueueThreadPipeline
    {
    private:
        AMFQueueThreadPipeline(const AMFQueueThreadPipeline&);
        AMFQueueThreadPipeline& operator=(const AMFQueueThreadPipeline&);

    public:
        AMFQueue<inT>* m_pInQueue;
        AMFQueue<outT>* m_pOutQueue;
        std::vector<_Thread*>    m_ThreadPool;

        AMFQueueThreadPipeline(AMFQueue<inT>* pInQueue, AMFQueue<outT>* pOutQueue)
            : m_pInQueue(pInQueue),
            m_pOutQueue(pOutQueue),
            m_ThreadPool()
        {}
        virtual ~AMFQueueThreadPipeline()
        {
            Stop();
        }
        void Start(int iNumberOfThreads, ThreadParam param)
        {
            if((long)m_ThreadPool.size() >= iNumberOfThreads)
            {
                Stop();  //temporary to remove stopped threads. need callback from thread to clean pool
                //return;
            }
            size_t initialSize = m_ThreadPool.size();
            for(size_t i = initialSize; i < (size_t)iNumberOfThreads; i++)
            {
                _Thread* pThread = new _Thread(m_pInQueue, m_pOutQueue, param);
                m_ThreadPool.push_back(pThread);
                pThread->Start();
            }
        }
        void RequestStop()
        {
            long num = (long)m_ThreadPool.size();
            for(long i = 0; i < num; i++)
            {
                m_ThreadPool[i]->RequestStop();
            }
        }
        void BlockProcessing()
        {
            long num = (long)m_ThreadPool.size();
            for(long i = 0; i < num; i++)
            {
                m_ThreadPool[i]->BlockProcessing();
            }
        }
        void UnblockProcessing()
        {
            long num = (long)m_ThreadPool.size();
            for(long i = 0; i < num; i++)
            {
                m_ThreadPool[i]->UnblockProcessing();
            }
        }
        void WaitForStop()
        {
            long num = (long)m_ThreadPool.size();
            for(long i = 0; i < num; i++)
            {
                _Thread* pThread = m_ThreadPool[i];
                pThread->WaitForStop();
                delete pThread;
            }
            m_ThreadPool.clear();
        }
        void Stop()
        {
            RequestStop();
            WaitForStop();
        }
    };
    //----------------------------------------------------------------
    class AMFPreciseWaiter
    {
    public:
        AMFPreciseWaiter() : m_WaitEvent(), m_bCancel(false)
        {}
        virtual ~AMFPreciseWaiter()
        {}
        amf_pts Wait(amf_pts waittime)
        {
            m_bCancel = false;
            amf_pts start = amf_high_precision_clock();
            amf_pts waited = 0;
            int count = 0; 
            while(!m_bCancel)
            {
                count++;
                if(!m_WaitEvent.LockTimeout(1))
                {
                    break;
                }
                waited = amf_high_precision_clock() - start;
                if(waited >= waittime)
                {
                    break;
                }
            }
            return waited;
        }
        void Cancel()
        {
            m_bCancel = true;
        }
    protected:
        AMFEvent m_WaitEvent;
        bool m_bCancel;
    };
    //----------------------------------------------------------------
} // namespace amf
#endif // __AMFThread_h__
