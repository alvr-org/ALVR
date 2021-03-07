//
// Copyright (c) 2017 Advanced Micro Devices, Inc. All rights reserved.
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

#ifndef AMF_CurrentTimeImpl_h
#define AMF_CurrentTimeImpl_h

#include "../include/core/CurrentTime.h"
#include "../common/InterfaceImpl.h"
#include "../common/Thread.h"

namespace amf
{

class AMFCurrentTimeImpl : public AMFInterfaceImpl<AMFCurrentTime>
{
public:
	AMFCurrentTimeImpl();
	~AMFCurrentTimeImpl();

	AMF_BEGIN_INTERFACE_MAP
		AMF_INTERFACE_ENTRY(AMFCurrentTime)
	AMF_END_INTERFACE_MAP

	virtual amf_pts AMF_STD_CALL Get();

	virtual void AMF_STD_CALL Reset();

private:
	amf_pts									m_timeOfFirstCall;
	mutable AMFCriticalSection				m_sync;
};

//----------------------------------------------------------------------------------------------
// smart pointer
//----------------------------------------------------------------------------------------------
typedef AMFInterfacePtr_T<AMFCurrentTime> AMFCurrentTimePtr;
//----------------------------------------------------------------------------------------------}
}
#endif // AMF_CurrentTimeImpl_h