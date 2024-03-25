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

/**
 ***************************************************************************************************
 * @file  ObservableImpl.h
 * @brief AMFObservableImpl common template declaration
 ***************************************************************************************************
 */
#ifndef AMF_ObservableImpl_h
#define AMF_ObservableImpl_h
#pragma once

#include "Thread.h"
#include <list>

namespace amf
{
    template<typename Observer>
    class AMFObservableImpl
    {
    private:
        typedef std::list<Observer*> ObserversList;
        ObserversList m_observers;
    public:
        AMFObservableImpl() : m_observers()
        {}
        virtual ~AMFObservableImpl()
        {
            assert(m_observers.size() == 0);
        }
        virtual void AMF_STD_CALL AddObserver(Observer* pObserver)
        {
            if (pObserver == nullptr)
            {
                return;
            }

            amf_bool found = false;
            AMFLock lock(&m_sc);
            
            for (typename ObserversList::iterator it = m_observers.begin(); it != m_observers.end(); it++)
            {
                if (*it == pObserver)
                {
                    found = true;
                    break;
                }
            }
            if (found == false)
            {
                m_observers.push_back(pObserver);
            }
        }

        virtual void AMF_STD_CALL RemoveObserver(Observer* pObserver)
        {
            AMFLock lock(&m_sc);
            m_observers.remove(pObserver);
        }

    protected:
        void AMF_STD_CALL ClearObservers()
        {
            AMFLock lock(&m_sc);
            m_observers.clear();
        }

        void AMF_STD_CALL NotifyObservers(void  (AMF_STD_CALL Observer::* pEvent)())
        {
            ObserversList tempList;
            {
                AMFLock lock(&m_sc);
                tempList = m_observers;
            }
            for (typename ObserversList::iterator it = tempList.begin(); it != tempList.end(); ++it)
            {
                Observer* pObserver = *it;
                (pObserver->*pEvent)();
            }
        }

        template<typename TArg0>
        void AMF_STD_CALL NotifyObservers(void (AMF_STD_CALL Observer::* pEvent)(TArg0), TArg0 arg0)
        {
            ObserversList tempList;
            {
                AMFLock lock(&m_sc);
                tempList = m_observers;
            }
            for (typename ObserversList::iterator it = tempList.begin(); it != tempList.end(); ++it)
            {
                Observer* pObserver = *it;
                (pObserver->*pEvent)(arg0);
            }
        }
        template<typename TArg0, typename TArg1>
        void AMF_STD_CALL NotifyObservers(void (AMF_STD_CALL Observer::* pEvent)(TArg0, TArg1), TArg0 arg0, TArg1 arg1)
        {
            ObserversList tempList;
            {
                AMFLock lock(&m_sc);
                tempList = m_observers;
            }
            for (typename ObserversList::iterator it = tempList.begin(); it != tempList.end(); it++)
            {
                Observer* pObserver = *it;
                (pObserver->*pEvent)(arg0, arg1);
            }
        }
    private:
        AMFCriticalSection m_sc;
    };
}
#endif //AMF_ObservableImpl_h
