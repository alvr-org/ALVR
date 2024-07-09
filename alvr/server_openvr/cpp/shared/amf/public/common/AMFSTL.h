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

#ifndef AMF_AMFSTL_h
#define AMF_AMFSTL_h

#pragma once

#if defined(__GNUC__)
    //disable gcc warinings on STL code
    #pragma GCC diagnostic push
    #pragma GCC diagnostic ignored "-Weffc++"
    #include <memory>  //default stl allocator
#else

    #include <xmemory>  //default stl allocator
#endif

#include <algorithm>
#include <string>
#include <vector>
#include <list>
#include <deque>
#include <queue>
#include <map>
#include <set>

#include "../include/core/Interface.h"

#if defined(__cplusplus)
extern "C"
{
#endif
    // allocator
    void* AMF_STD_CALL amf_alloc(amf_size count);
    void AMF_STD_CALL amf_free(void* ptr);
    void* AMF_STD_CALL amf_aligned_alloc(size_t count, size_t alignment);
    void AMF_STD_CALL amf_aligned_free(void* ptr);
#if defined(__cplusplus)
}
#endif

namespace amf
{
#pragma warning(push)

#pragma warning(disable: 4996)    // was declared deprecated
    //-------------------------------------------------------------------------------------------------
    // STL allocator redefined - will allocate all memory in "C" runtime of Common.DLL
    //-------------------------------------------------------------------------------------------------
    template<class _Ty>
    class amf_allocator : public std::allocator<_Ty>
    {
    public:
        amf_allocator() : std::allocator<_Ty>()
        {}
        amf_allocator(const amf_allocator<_Ty>& rhs) : std::allocator<_Ty>(rhs)
        {}
        template<class _Other> amf_allocator(const amf_allocator<_Other>& rhs) : std::allocator<_Ty>(rhs)
        {}
        template<class _Other> struct rebind // convert an allocator<_Ty> to an allocator <_Other>
        {
            typedef amf_allocator<_Other> other;
        };
        void deallocate(_Ty* const _Ptr, const size_t _Count)
        {
            _Count;
            amf_free((void*)_Ptr);
        }
        _Ty* allocate(const size_t _Count, const void* = static_cast<const void*>(0))
        { // allocate array of _Count el ements
            return static_cast<_Ty*>(amf_alloc(_Count * sizeof(_Ty)));
        }
    };




    //-------------------------------------------------------------------------------------------------
    // STL container templates with changed memory allocation
    //-------------------------------------------------------------------------------------------------
    template<class _Ty>
    class amf_vector
        : public std::vector<_Ty, amf_allocator<_Ty> >
    {
    public:
    typedef std::vector<_Ty, amf_allocator<_Ty> > _base;

    amf_vector() : _base() {}
    explicit amf_vector(size_t _Count) : _base(_Count) {} //MM GCC has strange compile error. to get around replaced size_type with size_t
    amf_vector(size_t _Count, const _Ty& _Val) : _base(_Count,_Val) {}
    };

    template<class _Ty>
    class amf_list
        : public std::list<_Ty, amf_allocator<_Ty> >
    {};

    template<class _Ty>
    class amf_deque
        : public std::deque<_Ty, amf_allocator<_Ty> >
    {};

    template<class _Ty>
    class amf_queue
        : public std::queue<_Ty, amf_deque<_Ty> >
    {};

    template<class _Kty, class _Ty, class _Pr = std::less<_Kty> >
    class amf_map
        : public std::map<_Kty, _Ty, _Pr, amf_allocator<std::pair<const _Kty, _Ty>> >
    {};

    template<class _Kty, class _Pr = std::less<_Kty> >
    class amf_set
        : public std::set<_Kty, _Pr, amf_allocator<_Kty> >
    {};

    template<class _Ty>
    class amf_limited_deque
        : public amf_deque<_Ty> // circular queue of pointers to blocks
    {
    public:
        typedef amf_deque<_Ty> _base;
        amf_limited_deque(size_t size_limit) : _base(), _size_limit(size_limit)
        {    // construct empty deque
        }
        size_t size_limit()
        {
            return _size_limit;
        }

        void set_size_limit(size_t size_limit)
        {
            _size_limit = size_limit;
            while(_base::size() > _size_limit)
            {
                _base::pop_front();
            }
        }

        _Ty push_front(const _Ty& _Val)
        {    // insert element at beginning
            _Ty ret;
            if(_size_limit > 0)
            {
                _base::push_front(_Val);
                if(_base::size() > _size_limit)
                {
                    ret = _base::back();
                    _base::pop_back();
                }
            }
            return ret;
        }
        void push_front_ex(const _Ty& _Val)
        {    // insert element at beginning
            _base::push_front(_Val);
        }

        _Ty push_back(const _Ty& _Val)
        {    // insert element at beginning
            _Ty ret;
            if(_size_limit > 0)
            {
                _base::push_back(_Val);
                if(_base::size() > _size_limit)
                {
                    ret = _base::front();
                    _base::pop_front();
                }
            }
            return ret;
        }

    protected:
        size_t _size_limit;
    };
#pragma warning(pop)
    //---------------------------------------------------------------
#if defined(__GNUC__)
    //disable gcc warinings on STL code
    #pragma GCC diagnostic push
    #pragma GCC diagnostic ignored "-Weffc++"
#endif

    template<class _Interf>
    class AMFInterfacePtr_TAdapted : public AMFInterfacePtr_T<_Interf>
    {
    public:
        AMFInterfacePtr_TAdapted* operator&()
        {
            return this;
        }

        AMFInterfacePtr_TAdapted()
            : AMFInterfacePtr_T<_Interf>()
        {}

        AMFInterfacePtr_TAdapted(_Interf* pOther)
            : AMFInterfacePtr_T<_Interf>(pOther)
        {}

        AMFInterfacePtr_TAdapted(const AMFInterfacePtr_T<_Interf>& other)
            : AMFInterfacePtr_T<_Interf>(other)
        {}
    };

    template<class _Interf>
    class amf_vector<AMFInterfacePtr_T<_Interf> >
        : public std::vector<AMFInterfacePtr_TAdapted<_Interf>, amf_allocator<AMFInterfacePtr_TAdapted<_Interf> > >
    {
    public:
        typedef AMFInterfacePtr_T<_Interf>& reference;
        typedef std::vector<AMFInterfacePtr_TAdapted<_Interf>, amf_allocator<AMFInterfacePtr_TAdapted<_Interf> > > baseclass;
        reference operator[](size_t n)
        {
            return baseclass::operator[](n);
        }
    };

    template<class _Interf>
    class amf_deque<AMFInterfacePtr_T<_Interf> >
        : public std::deque<AMFInterfacePtr_TAdapted<_Interf>, amf_allocator<AMFInterfacePtr_TAdapted<_Interf> > >
    {};

    template<class _Interf>
    class amf_list<AMFInterfacePtr_T<_Interf> >
        : public std::list<AMFInterfacePtr_TAdapted<_Interf>, amf_allocator<AMFInterfacePtr_TAdapted<_Interf> > >
    {};
#if defined(__GNUC__)
    // restore gcc warnings
    #pragma GCC diagnostic pop
#endif
}
//-------------------------------------------------------------------------------------------------
// string classes
//-------------------------------------------------------------------------------------------------

typedef std::basic_string<char, std::char_traits<char>, amf::amf_allocator<char> > amf_string;
typedef std::basic_string<wchar_t, std::char_traits<wchar_t>, amf::amf_allocator<wchar_t> > amf_wstring;

template <class TAmfString>
std::size_t amf_string_hash(TAmfString const& s) noexcept
{
#if defined(_WIN64) || defined(__x86_64__)
    constexpr size_t fnvOffsetBasis = 14695981039346656037ULL;
    constexpr size_t fnvPrime = 1099511628211ULL;
#else // defined(_WIN64) || defined(__x86_64__)
    constexpr size_t fnvOffsetBasis = 2166136261U;
    constexpr size_t fnvPrime = 16777619U;
#endif // defined(_WIN64) || defined(__x86_64__)

    const unsigned char* const pStr = reinterpret_cast<const unsigned char*>(s.c_str());
    const size_t count = s.size() * sizeof(typename TAmfString::value_type);
    size_t value = fnvOffsetBasis;
    for (size_t i = 0; i < count; ++i)
    {
        value ^= static_cast<size_t>(pStr[i]);
        value *= fnvPrime;
    }
    return value;
}

template<>
struct std::hash<amf_wstring>
{
    std::size_t operator()(amf_wstring const& s) const noexcept
    {
        return amf_string_hash<amf_wstring>(s);
    }
};

template<>
struct std::hash<amf_string>
{
    std::size_t operator()(amf_string const& s) const noexcept
    {
        return amf_string_hash<amf_string>(s);
    }
};

namespace amf
{
    //-------------------------------------------------------------------------------------------------
    // string conversion
    //-------------------------------------------------------------------------------------------------
    amf_string AMF_STD_CALL amf_from_unicode_to_utf8(const amf_wstring& str);
    amf_wstring AMF_STD_CALL amf_from_utf8_to_unicode(const amf_string& str);
    amf_string AMF_STD_CALL amf_from_unicode_to_multibyte(const amf_wstring& str);
    amf_wstring AMF_STD_CALL amf_from_multibyte_to_unicode(const amf_string& str);
    amf_string AMF_STD_CALL amf_from_string_to_hex_string(const amf_string& str);
    amf_string AMF_STD_CALL amf_from_hex_string_to_string(const amf_string& str);

    amf_string AMF_STD_CALL amf_string_to_lower(const amf_string& str);
    amf_wstring AMF_STD_CALL amf_string_to_lower(const amf_wstring& str);
    amf_string AMF_STD_CALL amf_string_to_upper(const amf_string& str);
    amf_wstring AMF_STD_CALL amf_string_to_upper(const amf_wstring& str);

    amf_string AMF_STD_CALL amf_from_unicode_to_url_utf8(const amf_wstring& data, bool bQuery = false); // converts to UTF8 and replace fobidden symbols
    amf_wstring AMF_STD_CALL amf_from_url_utf8_to_unicode(const amf_string& data);

    amf_wstring AMF_STD_CALL amf_convert_path_to_os_accepted_path(const amf_wstring& path);
    amf_wstring AMF_STD_CALL amf_convert_path_to_url_accepted_path(const amf_wstring& path);

    //-------------------------------------------------------------------------------------------------
    // string helpers
    //-------------------------------------------------------------------------------------------------
    amf_wstring AMF_STD_CALL amf_string_format(const wchar_t* format, ...);
    amf_string AMF_STD_CALL amf_string_format(const char* format, ...);

    amf_wstring AMF_STD_CALL amf_string_formatVA(const wchar_t* format, va_list args);
    amf_string AMF_STD_CALL amf_string_formatVA(const char* format, va_list args);

    amf_int AMF_STD_CALL amf_string_ci_compare(const amf_wstring& left, const amf_wstring& right);
    amf_int AMF_STD_CALL amf_string_ci_compare(const amf_string& left, const amf_string& right);

    amf_size AMF_STD_CALL amf_string_ci_find(const amf_wstring& left, const amf_wstring& right, amf_size off = 0);
    amf_size AMF_STD_CALL amf_string_ci_rfind(const amf_wstring& left, const amf_wstring& right, amf_size off = amf_wstring::npos);
    //-------------------------------------------------------------------------------------------------
} // namespace amf




#if defined(__GNUC__)
    // restore gcc warnings
    #pragma GCC diagnostic pop
#endif

#endif // AMF_AMFSTL_h

