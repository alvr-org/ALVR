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

#include "CurrentTimeImpl.h"

namespace amf
{

	//-------------------------------------------------------------------------------------------------
	AMFCurrentTimeImpl::AMFCurrentTimeImpl()
		: m_timeOfFirstCall(-1)
	{
	}

	//-------------------------------------------------------------------------------------------------
	AMFCurrentTimeImpl::~AMFCurrentTimeImpl()
	{
		m_timeOfFirstCall = -1;
	}

	//-------------------------------------------------------------------------------------------------
	amf_pts AMF_STD_CALL AMFCurrentTimeImpl::Get()
	{
		amf::AMFLock lock(&m_sync);

		// We want pts time to start at 0 and subsequent
		// times to be relative to that
		if (m_timeOfFirstCall < 0)
		{
			m_timeOfFirstCall = amf_high_precision_clock();
			return 0;
		}
		return (amf_high_precision_clock() - m_timeOfFirstCall); // In nanoseconds
	}

	//-------------------------------------------------------------------------------------------------
	void AMF_STD_CALL AMFCurrentTimeImpl::Reset()
	{
		m_timeOfFirstCall = -1;
	}
}