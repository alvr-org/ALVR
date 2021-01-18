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

#include "AMFSTL.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <locale>
#include <locale.h>

#pragma warning(disable: 4996)

using namespace amf;

static const amf_string AMF_FORBIDDEN_SYMBOLS = ":? %,;@&=+$<>#\"";
static const amf_string AMF_FORBIDDEN_SYMBOLS_QUERY = ":? %,;@+$<>#\"";

//MM: in question:  unwise      = "{" | "}" | "|" | "\" | "^" | "[" | "]" | "`"

//----------------------------------------------------------------------------------------
// string conversaion
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_from_unicode_to_utf8(const amf_wstring& str)
{
    amf_string result;
    if(0 == str.size())
    {
        return result;
    }
#if defined(_WIN32)
    _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
#endif


    const wchar_t* pwBuff = str.c_str();

#if defined(_WIN32)
    int Utf8BuffSize = ::WideCharToMultiByte(CP_UTF8, 0, pwBuff, -1, NULL, 0, NULL, NULL);
    if(0 == Utf8BuffSize)
    {
        return result;
    }
    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);
    Utf8BuffSize = ::WideCharToMultiByte(CP_UTF8, 0, pwBuff, -1, &result[0], Utf8BuffSize, NULL, NULL);
    Utf8BuffSize--;
#elif defined(__ANDROID__)
    char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
    int Utf8BuffSize = str.length();
    if(0 == Utf8BuffSize)
    {
        return result;
    }
    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);

    mbstate_t mbs;
    mbrlen(NULL, 0, &mbs);

    Utf8BuffSize = 0;
    for( int i = 0; i < str.length(); i++)
    {
        //MM TODO Android - not implemented
        //int written = wcrtomb(&result[Utf8BuffSize], pwBuff[i], &mbs);
        result[Utf8BuffSize] = (char)(pwBuff[i]);
        int written = 1;
        // temp replacement
        Utf8BuffSize += written;
    }
    setlocale(LC_CTYPE, old_locale);

#else
    char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
    int Utf8BuffSize = wcstombs(NULL, pwBuff, 0);
    if(0 == Utf8BuffSize)
    {
        return result;
    }
    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);
    Utf8BuffSize = wcstombs(&result[0], pwBuff, Utf8BuffSize);

    setlocale(LC_CTYPE, old_locale);
#endif
    result.resize(Utf8BuffSize);


    return result;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_from_utf8_to_unicode(const amf_string& str)
{
    amf_wstring result;
    if(0 == str.size())
    {
        return result;
    }
#if defined(_WIN32)
    _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
#endif


    const char* pUtf8Buff = str.c_str();

#if defined(_WIN32)
    int UnicodeBuffSize = ::MultiByteToWideChar(CP_UTF8, 0, pUtf8Buff, -1, NULL, 0);
    if(0 == UnicodeBuffSize)
    {
        return result;
    }
    UnicodeBuffSize += 8; // get some extra space
    result.resize(UnicodeBuffSize);
    UnicodeBuffSize = ::MultiByteToWideChar(CP_UTF8, 0, pUtf8Buff, -1, &result[0], UnicodeBuffSize);
    UnicodeBuffSize--;

#elif defined(__ANDROID__)
    //MM on android mbstowcs cannot be used to define length
    char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");

    mbstate_t mbs;
    mbrlen(NULL, 0, &mbs);
    int len = str.length();
    const char* pt = pUtf8Buff;
    int UnicodeBuffSize = 0;
    while(len > 0)
    {
        size_t length = mbrlen (pt, len, &mbs); //MM TODO Android always return 1
        if((length == 0) || (length > len))
        {
            break;
        }
        UnicodeBuffSize++;
        len -= length;
        pt += length;
    }
    UnicodeBuffSize += 8; // get some extra space
    result.resize(UnicodeBuffSize);

    mbrlen (NULL, 0, &mbs);
    len = str.length();
    pt = pUtf8Buff;
    UnicodeBuffSize = 0;
    while(len > 0)
    {
        size_t length = mbrlen (pt, len, &mbs);
        if((length == 0) || (length > len))
        {
            break;
        }
        mbrtowc(&result[UnicodeBuffSize], pt, length, &mbs);     //MM TODO Android always return 1 char
        UnicodeBuffSize++;
        len -= length;
        pt += length;
    }
    setlocale(LC_CTYPE, old_locale);

 #else
    char* old_locale = setlocale(LC_CTYPE, "en_US.UTF8");
    int UnicodeBuffSize = mbstowcs(NULL, pUtf8Buff, 0);
    if(0 == UnicodeBuffSize)
    {
        return result;
    }
    UnicodeBuffSize += 8; // get some extra space
    result.resize(UnicodeBuffSize);
    UnicodeBuffSize = mbstowcs(&result[0], pUtf8Buff, UnicodeBuffSize);
    setlocale(LC_CTYPE, old_locale);
#endif
    result.resize(UnicodeBuffSize);


    return result;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_from_unicode_to_multibyte(const amf_wstring& str)
{
    amf_string result;
    if(0 == str.size())
    {
        return result;
    }

    const wchar_t* pwBuff = str.c_str();

#if defined(__ANDROID__)
    int Utf8BuffSize = str.length();
    if(0 == Utf8BuffSize)
    {
        return result;
    }
    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);

    mbstate_t mbs;
    mbrlen(NULL, 0, &mbs);

    Utf8BuffSize = 0;
    for( int i = 0; i < str.length(); i++)
    {
        //MM TODO Android - not implemented
        //int written = wcrtomb(&result[Utf8BuffSize], pwBuff[i], &mbs);
        result[Utf8BuffSize] = (char)(pwBuff[i]);
        int written = 1;
        // temp replacement
        Utf8BuffSize += written;
    }
#else
    amf_size Utf8BuffSize = wcstombs(NULL, pwBuff, 0);
    if(0 == Utf8BuffSize)
    {
        return result;
    }

    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);
    Utf8BuffSize = wcstombs(&result[0], pwBuff, Utf8BuffSize);
#endif
    result.resize(Utf8BuffSize);
    return result;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_from_multibyte_to_unicode(const amf_string& str)
{
    amf_wstring result;
    if(0 == str.size())
    {
        return result;
    }

    const char* pUtf8Buff = str.c_str();


#if defined(__ANDROID__)
    //MM on android mbstowcs cannot be used to define length
    mbstate_t mbs;
    mbrlen(NULL, 0, &mbs);
    int len = str.length();
    const char* pt = pUtf8Buff;
    int UnicodeBuffSize = 0;
    while(len > 0)
    {
        size_t length = mbrlen (pt, len, &mbs); //MM TODO Android always return 1
        if((length == 0) || (length > len))
        {
            break;
        }
        UnicodeBuffSize++;
        len -= length;
        pt += length;
    }
    UnicodeBuffSize += 8; // get some extra space
    result.resize(UnicodeBuffSize);

    mbrlen (NULL, 0, &mbs);
    len = str.length();
    pt = pUtf8Buff;
    UnicodeBuffSize = 0;
    while(len > 0)
    {
        size_t length = mbrlen (pt, len, &mbs);
        if((length == 0) || (length > len))
        {
            break;
        }
        mbrtowc(&result[UnicodeBuffSize], pt, length, &mbs);     //MM TODO Android always return 1 char
        UnicodeBuffSize++;
        len -= length;
        pt += length;
    }
 #else
    amf_size UnicodeBuffSize = mbstowcs(NULL, pUtf8Buff, 0);
    if(0 == UnicodeBuffSize)
    {
        return result;
    }

    UnicodeBuffSize += 8; // get some extra space
    result.resize(UnicodeBuffSize);
    UnicodeBuffSize = mbstowcs(&result[0], pUtf8Buff, UnicodeBuffSize);
#endif
    result.resize(UnicodeBuffSize);
    return result;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_from_string_to_hex_string(const amf_string& str)
{
    amf_string ret;
    char buf[10];
    for(int i = 0; i < (int)str.length(); i++)
    {
        sprintf(buf, "%02X", (unsigned char)str[i]);
        ret += buf;
    }

    return ret;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_from_hex_string_to_string(const amf_string& str)
{
    amf_string ret;
    char buf[3] = {
        0, 0, 0
    };
    for(int i = 0; i < (int)str.length(); i += 2)
    {
        buf[0] = str[i];
        buf[1] = str[i + 1];
        int tmp = 0;
        sscanf(buf, "%2X", &tmp);
        ret += (char)tmp;
    }
    return ret;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_string_to_lower(const amf_string& str)
{
    std::locale loc;
    amf_string out = str.c_str();
    size_t iLen = out.length();
    for(size_t i = 0; i < iLen; i++)
    {
        out[i] = std::tolower (out[i], loc);
    }
    return out;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_string_to_lower(const amf_wstring& str)
{
    std::locale loc;
    amf_wstring out = str.c_str();
    size_t iLen = out.length();
    for(size_t i = 0; i < iLen; i++)
    {
        out[i] = std::tolower (out[i], loc);
    }
    return out;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_string_to_upper(const amf_string& str)
{
    std::locale loc;
    amf_string out = str.c_str();
    size_t iLen = out.length();
    for(size_t i = 0; i < iLen; i++)
    {
        out[i] = std::toupper (out[i], loc);
    }
    return out;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_string_to_upper(const amf_wstring& str)
{
    std::locale loc;
    amf_wstring out = str.c_str();
    size_t iLen = out.length();
    for(size_t i = 0; i < iLen; i++)
    {
        out[i] = std::toupper (out[i], loc);
    }
    return out;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_convert_path_to_os_accepted_path(const amf_wstring& path)
{
    amf_wstring result = path;
    amf_wstring::size_type pos = 0;
    while(pos != amf_string::npos)
    {
        pos = result.find(L'/', pos);
        if(pos == amf_wstring::npos)
        {
            break;
        }
        result[pos] = PATH_SEPARATOR_WCHAR;
        pos++;
    }
    return result;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_convert_path_to_url_accepted_path(const amf_wstring& path)
{
    amf_wstring result = path;
    amf_wstring::size_type pos = 0;
    while(pos != amf_string::npos)
    {
        pos = result.find(L'\\', pos);
        if(pos == amf_wstring::npos)
        {
            break;
        }
        result[pos] = L'/';
        pos++;
    }
    return result;
}
//----------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_from_unicode_to_url_utf8(const amf_wstring& data, bool bQuery) // converts to UTF8 and replace fobidden symbols
{
    amf_string converted = amf_from_unicode_to_utf8(amf_convert_path_to_url_accepted_path(data));
    // convert all necessary symbols to hex
    amf_string Result;

    amf_size num = converted.length();
    char buf[20];
    for(amf_size i = 0; i < num; i++)
    {
        if((converted[i] <= 0x20) || (converted[i] >= 0x7F) ||
           (bQuery && ( AMF_FORBIDDEN_SYMBOLS.find(converted[i]) != amf_string::npos) ) ||
           (!bQuery && ( AMF_FORBIDDEN_SYMBOLS_QUERY.find(converted[i]) != amf_string::npos) ))
        {
            _snprintf(buf, 20, "%%%02X", (unsigned int)(unsigned char)converted[i]);
        }
        else
        {
            buf[0] = converted[i];
            buf[1] = 0;
        }
        Result += buf;
    }
/*
    amf_string::size_type pos=0;
    while(true){
        amf_string::size_type old_pos=pos;
        if(bQuery)
            pos=converted.find_first_of(MM_FORBIDDEN_SYMBOLS,pos);
        else
            pos=converted.find_first_of(MM_FORBIDDEN_SYMBOLS_QUERY,pos);
        if(pos==amf_string::npos){
            Result+=converted.substr(old_pos);
            break;
        }
        if(pos-old_pos>0)
            Result+=converted.substr(old_pos,pos-old_pos);
        char buf[20];
        _snprintf(buf,20,"%%%02X",(int)converted[pos]);
        Result+=buf;
        pos++;
    }
 */
    return Result;
}
//------------------------------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_from_url_utf8_to_unicode(const amf_string& data)
{
    amf_string Result;
    amf_string::size_type pos = 0;
    while(pos != amf_string::npos)
    {
        amf_string::size_type old_pos = pos;
        pos = data.find('%', pos);
        if(pos == amf_string::npos)
        {
            Result += data.substr(old_pos);
            break;
        }
        if(pos - old_pos > 0)
        {
            Result += data.substr(old_pos, pos - old_pos);
        }
        char buf[5] = {
            '0', 'x', 0, 0, 0
        };
        buf[2] = data[pos + 1];
        buf[3] = data[pos + 2];
        char* ret = NULL;

        Result += (char)strtol(buf, &ret, 16);
        pos += 3;
    }

    amf_wstring converted = amf_from_utf8_to_unicode(Result);

    return converted;
}
//----------------------------------------------------------------------------------------------
amf_size AMF_STD_CALL amf::amf_string_ci_find(const amf_wstring& left, const amf_wstring& right, amf_size off)
{
    amf_wstring _left = amf_string_to_lower(left);
    amf_wstring _right = amf_string_to_lower(right);
    return _left.find(_right, off);
}
//----------------------------------------------------------------------------------------------
amf_size AMF_STD_CALL amf::amf_string_ci_rfind(const amf_wstring& left, const amf_wstring& right, amf_size off)
{
    amf_wstring _left = amf_string_to_lower(left);
    amf_wstring _right = amf_string_to_lower(right);
    return _left.rfind(_right, off);
}
//----------------------------------------------------------------------------------------------
amf_int AMF_STD_CALL amf::amf_string_ci_compare(const amf_wstring& left, const amf_wstring& right)
{
    amf_wstring _left = amf_string_to_lower(left);
    amf_wstring _right = amf_string_to_lower(right);
    return _left.compare(_right);
}
//----------------------------------------------------------------------------------------------
amf_int AMF_STD_CALL amf::amf_string_ci_compare(const amf_string& left, const amf_string& right)
{
    amf_string _left = amf_string_to_lower(left);
    amf_string _right = amf_string_to_lower(right);
    return _left.compare(_right);
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_string_format(const wchar_t* format, ...)
{
    va_list arglist;
    va_start(arglist, format);
    amf_wstring text = amf_string_formatVA(format, arglist);
    va_end(arglist);

    return text;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_string_format(const char* format, ...)
{
    va_list arglist;
    va_start(arglist, format);
    amf_string text = amf_string_formatVA(format, arglist);
    va_end(arglist);

    return text;
}
//----------------------------------------------------------------------------------------
amf_wstring AMF_STD_CALL amf::amf_string_formatVA(const wchar_t* format, va_list args)
{
#if defined(__linux)
    //replace %s with %ls
    amf_wstring text(format);
    amf_wstring textReplaced;
    textReplaced.reserve(text.length() * 2);
    bool percentFlag = false;
    for(amf_wstring::iterator i = text.begin(); i != text.end(); ++i)
    {
        if(percentFlag && (*i == L's'))
        {
            textReplaced.push_back(L'l');
            textReplaced.push_back(L's');
        }
        else
        {
            textReplaced.push_back(*i);
        }
        percentFlag = (*i != L'%') ? false : !percentFlag;
    }

    format = textReplaced.c_str();
#endif //#if defined(__linux)
    va_list argcopy;
#ifdef _WIN32
    argcopy = args;
#else
    va_copy(argcopy, args);
#endif

    int size = _vscwprintf(format, argcopy);
    va_end(argcopy);


    std::vector<wchar_t> buf(size + 1);
    wchar_t* pBuf = &buf[0];
    _vsnwprintf(pBuf, size + 1, format, args);
    return pBuf;
}
//----------------------------------------------------------------------------------------
amf_string AMF_STD_CALL amf::amf_string_formatVA(const char* format, va_list args)
{
    va_list argcopy;

#ifdef _WIN32
    argcopy = args;
#else
    va_copy(argcopy, args);
#endif

    int size = _vscprintf(format, args);
    va_end(argcopy);

    std::vector<char> buf(size + 1);
    char* pBuf = &buf[0];
    vsnprintf(pBuf, size + 1, format, args);
    return pBuf;
}
//----------------------------------------------------------------------------------------
void* AMF_STD_CALL amf_alloc(size_t count)
{
    return malloc(count);
}
//----------------------------------------------------------------------------------------
void AMF_STD_CALL amf_free(void* ptr)
{
    free(ptr);
}
