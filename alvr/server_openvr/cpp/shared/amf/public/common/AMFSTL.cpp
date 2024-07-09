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

#include "AMFSTL.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <locale>
#include <locale.h>
#include <fstream>
#include <string>
#include <wchar.h>
#include <stdarg.h>
#if defined(__ANDROID__)
    #include <codecvt>
#endif

#if !defined(__APPLE__) && !defined(_WIN32)
#include <malloc.h>
#endif

#pragma warning(disable: 4996)

#if defined(__linux) || defined(__APPLE__)
extern "C"
{
    extern int vscwprintf(const wchar_t* p_fmt, va_list p_args);
    extern int vscprintf(const char* p_fmt, va_list p_args);
}
#endif

#ifdef _MSC_VER
    #define snprintf _snprintf
    #define vscprintf _vscprintf
    #define vscwprintf _vscwprintf  //  Count chars without writing to string
    #define vswprintf _vsnwprintf
#endif


using namespace amf;

#ifdef __clang__
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wexit-time-destructors"
    #pragma clang diagnostic ignored "-Wglobal-constructors"
#endif

static const amf_string AMF_FORBIDDEN_SYMBOLS = ":? %,;@&=+$<>#\"";
static const amf_string AMF_FORBIDDEN_SYMBOLS_QUERY = ":? %,;@+$<>#\"";

#ifdef __clang__
    #pragma clang diagnostic pop
#endif


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
    std::wstring_convert<std::codecvt_utf8<wchar_t>> converter;
    result.assign(converter.to_bytes(pwBuff).c_str());
/*
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
    result.resize(Utf8BuffSize);
*/
#else
    amf_size Utf8BuffSize = wcstombs(NULL, pwBuff, 0);
    if(static_cast<std::size_t>(-1) == Utf8BuffSize)
    {
        return result;
    }

    Utf8BuffSize += 8; // get some extra space
    result.resize(Utf8BuffSize);
    Utf8BuffSize = wcstombs(&result[0], pwBuff, Utf8BuffSize);
    result.resize(Utf8BuffSize);
#endif
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
            snprintf(buf, sizeof(buf), "%%%02X", (unsigned int)(unsigned char)converted[i]);
        }
        else
        {
            buf[0] = converted[i];
            buf[1] = 0;
        }
        Result += buf;
    }
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
#if (defined(__linux) || defined(__APPLE__)) && (!defined(__ANDROID__))
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
        else if(percentFlag && (*i == L'S'))
        {
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
    int size = vscwprintf(format, argcopy);

    va_end(argcopy);


    std::vector<wchar_t> buf(size + 1);
    wchar_t* pBuf = &buf[0];
    vswprintf(pBuf, size + 1, format, args);
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
    int size = vscprintf(format, args);

    va_end(argcopy);

    std::vector<char> buf(size + 1);
    char* pBuf = &buf[0];
    vsnprintf(pBuf, size + 1, format, args);
    return pBuf;
}
#if (defined(__linux) || defined(__APPLE__)) && !defined(__ANDROID__)
int vscprintf(const char* format, va_list argptr)
{
    char* p_tmp_buf;
    size_t tmp_buf_size;
    FILE* fd = open_memstream(&p_tmp_buf, &tmp_buf_size);
    if(fd == 0)
    {
        return -1;
    }
    va_list arg_copy;
    va_copy(arg_copy, argptr);
    vfprintf(fd, format, arg_copy);
    va_end(arg_copy);
    fclose(fd);
    free(p_tmp_buf);
    return tmp_buf_size;
}

int vscwprintf(const wchar_t* format, va_list argptr)
{
    wchar_t* p_tmp_buf;
    size_t tmp_buf_size;
    FILE* fd = open_wmemstream(&p_tmp_buf, &tmp_buf_size);
    if(fd == 0)
    {
        return -1;
    }
    va_list arg_copy;
    va_copy(arg_copy, argptr);
    vfwprintf(fd, format, argptr);
    va_end(arg_copy);
    fclose(fd);
    free(p_tmp_buf);
    return tmp_buf_size;
}
#endif

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
//----------------------------------------------------------------------------------------
void* AMF_STD_CALL amf_aligned_alloc(size_t count, size_t alignment)
{
#if defined(_WIN32)
    return _aligned_malloc(count, alignment);
#elif defined (__APPLE__)
    void* p = nullptr;
    posix_memalign(&p, alignment, count);
    return p;
#elif defined(__linux)
    return memalign(alignment, count);
#endif
}
//----------------------------------------------------------------------------------------
void AMF_STD_CALL amf_aligned_free(void* ptr)
{
#if defined(_WIN32)
    return _aligned_free(ptr);
#else
    return free(ptr);
#endif
}
//----------------------------------------------------------------------------------------
#if defined (__ANDROID__)
template <typename CHAR_T>
static bool isOneOf(CHAR_T p_ch, const CHAR_T* p_set)
{
    for (const CHAR_T* current = p_set; *current != 0; ++current)
    {
        if (*current == p_ch)
            return true;
    }
    return false;
}

static void processWidthAndPrecision(amf_string& p_fmt, va_list& p_args)
{
    for (size_t i = 0; i < p_fmt.length(); i++)
    {
        if (p_fmt[i] == '*')
        {
            int value = va_arg(p_args, int);
            char valueString[64];
            sprintf(valueString, "%d", value);
            p_fmt.replace(i, 1, valueString);
        }
    }
}

typedef size_t(*outputStreamDelegateW)(void* p_context, size_t p_offset, const wchar_t* p_stringToAdd, size_t p_length);
static size_t amf_wprintfCore(outputStreamDelegateW p_outDelegate, void* p_context, const wchar_t* p_fmt, va_list p_args)
{
    static const wchar_t formatSpecifiers[] = L"cCdiouxXeEfgGaAnpsSZ";
    bool inFormat = false;
    const wchar_t* beginCurrentFormat = NULL;
    amf_wstring::size_type formatLength = 0;
    amf_wstring currentFormat;
    amf_wstring currentArgumentString;
    size_t totalCount = 0;

    for (const wchar_t* fmt = p_fmt; *fmt != L'\0'; ++fmt)
    {
        if (*fmt == L'%')
        {
            inFormat = !inFormat;
            if (inFormat)    //    Beginning of a format substring - fmt points at the opening %
            {
                beginCurrentFormat = fmt;    //    Save the pointer to the current format substring
                formatLength = 0;
            }
            else    //    This was a percent character %% - don't bother
            {
                beginCurrentFormat = NULL;
            }
            currentFormat.clear();
        }
        if (inFormat)
        {
            ++formatLength;

            if (isOneOf<wchar_t>(*fmt, formatSpecifiers))
            {    //    end of the format specifier
                inFormat = false;
                currentFormat.assign(beginCurrentFormat, formatLength);    //    currentFormat now contains a modified format string for the current parameter
                amf_string currentFormatMB = amf_from_unicode_to_multibyte(currentFormat.c_str());
                //processWidthAndPrecision(currentFormatMB, &p_args[0]);    //    This would extract additional arguments for width and precision and replace * with their values
                for (size_t i = 0; i < currentFormatMB.length(); i++)
                {
                    if (currentFormatMB[i] == '*')
                    {
                        int value = va_arg(p_args, int);
                        char valueString[64];
                        sprintf(valueString, "%d", value);
                        currentFormatMB.replace(i, 1, valueString);
                    }
                }

                switch (*fmt)
                {
                case L'c':
                {
                    wchar_t ch;
                    switch (*(fmt - 1))
                    {
                    case L'h':
                        ch = static_cast<wchar_t>(va_arg(p_args, int));
                        break;
                    case L'l':
                    case L'w':
                        ch = va_arg(p_args, unsigned int);
                        break;
                    default:
                        ch = va_arg(p_args, unsigned int);    //    In a wchar_t version of printf %c means wchar_t
                    }
                    currentArgumentString = ch;
                }
                break;
                case L'C':
                {
                    wchar_t ch;
                    switch (*(fmt - 1))
                    {
                    case L'h':
                        ch = static_cast<wchar_t>(va_arg(p_args, int));
                        break;
                    case L'l':
                    case L'w':
                        ch = va_arg(p_args, unsigned int);
                        break;
                    default:
                        ch = static_cast<wchar_t>(va_arg(p_args, int));    //    In a wchar_t version of printf %C means char
                    }
                    currentArgumentString = ch;
                }
                break;
                case L's':
                {
                    const void* str = va_arg(p_args, const void*);
                    if (str != NULL)
                    {
                        const wchar_t* str_wchar = nullptr;
                        switch (*(fmt - 1))
                        {
                        case L'h':
                            currentArgumentString = amf_from_utf8_to_unicode(reinterpret_cast<const char*>(str));
                            str_wchar = currentArgumentString.c_str();
                            break;
                        case L'l':
                        case L'w':
                            currentArgumentString = str_wchar = reinterpret_cast<const wchar_t*>(str);
                            break;
                        default:
                            currentArgumentString = str_wchar = reinterpret_cast<const wchar_t*>(str);
                        }
                    }
                    else
                    {
                        currentArgumentString = L"(null)";
                    }
                }
                break;
                case L'S':
                {
                    const void* str = va_arg(p_args, const void*);
                    if (str != NULL)
                    {
                        switch (*(fmt - 1))
                        {
                        case (wchar_t)'h':
                            currentArgumentString = amf_from_utf8_to_unicode(reinterpret_cast<const char*>(str));
                            break;
                        case L'l':
                        case L'w':
                            currentArgumentString = reinterpret_cast<const wchar_t*>(str);
                            break;
                        default:
                            currentArgumentString = amf_from_utf8_to_unicode(reinterpret_cast<const char*>(str));
                        }
                    }
                    else
                    {
                        currentArgumentString = L"(null)";
                    }
                }
                break;
                //    All integer formats
                case L'i':
                case L'd':
                case L'u':
                case L'o':
                case L'x':
                case L'X':
                {
                    char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                    switch (*(fmt - 1))
                    {
                    case L'l':
                        if (*(fmt - 2) == L'l')    //    long long
                        {
                            sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long long));
                        }
                        else
                        {
                            sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long));
                        }
                        break;
                    case L'h':
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, int));
                        break;
#ifdef _WIN32
                    case L'I':    //    I is Microsoft-specific
#else
                    case L'z':    //    z and t are C99-specific, but seem to be unsupported in VC
                    case L't':
#endif
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, size_t));
                        break;
                    default:
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, int));
                        break;
                    }
                    currentArgumentString = amf_from_utf8_to_unicode(tempBuffer);
                }
                break;
                //    All floating point formats
                case L'e':
                case L'E':
                case L'f':
                case L'g':
                case L'G':
                case L'a':
                case L'A':
                {
                    char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                    switch (*(fmt - 1))
                    {
                    case L'l':
                    case L'L':
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long double));
                        break;
                    default:
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, double));
                        break;
                    }
                    currentArgumentString = amf_from_utf8_to_unicode(tempBuffer);
                }
                break;
                //    Pointer
                case L'p':
                {
                    char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                    sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, const void*));
                    currentArgumentString = amf_from_utf8_to_unicode(tempBuffer);
                }
                break;
                case L'n':
                {
                    int* dest = va_arg(p_args, int*);
                    *dest = static_cast<int>(totalCount);
                    currentArgumentString.clear();
                }
                break;
                }
                size_t length = currentArgumentString.length();
                if (p_outDelegate != NULL)    // If destination buffer is NULL, just count the characters
                {
                    p_outDelegate(p_context, totalCount, currentArgumentString.c_str(), length);
                }
                totalCount += length;
            }    //    if (isOneOf(*fmt, formatSpecifiers))
        }    // if (inFormat)
        else
        {    //    Just copy the character into the output buffer
            if (p_outDelegate != NULL)    // If destination buffer is NULL, just count the characters
            {
                p_outDelegate(p_context, totalCount, fmt, 1);
            }
            ++totalCount;
        }
    }
    return totalCount;
}

typedef size_t(*outputStreamDelegate)(void* p_context, size_t p_offset, const char* p_stringToAdd, size_t p_length);
static size_t amf_printfCore(outputStreamDelegate p_outDelegate, void* p_context, const char* p_fmt, va_list p_args)
{
    static const char formatSpecifiers[] = "cCdiouxXeEfgGaAnpsSZ";
    bool inFormat = false;
    const char* beginCurrentFormat = NULL;
    amf_string::size_type formatLength = 0;
    amf_string currentFormat;
    amf_string currentArgumentString;
    size_t totalCount = 0;

    for (const char* fmt = p_fmt; *fmt != '\0'; ++fmt)
    {
        if (*fmt == '%')
        {
            inFormat = !inFormat;
            if (inFormat)    //    Beginning of a format substring - fmt points at the opening %
            {
                beginCurrentFormat = fmt;    //    Save the pointer to the current format substring
                formatLength = 0;
            }
            else    //    This was a percent character %% - don't bother
            {
                beginCurrentFormat = NULL;
            }
            currentFormat.clear();
        }
        if (inFormat)
        {
            ++formatLength;

            if (isOneOf<char>(*fmt, formatSpecifiers))
            {    //    end of the format specifier
                inFormat = false;
                currentFormat.assign(beginCurrentFormat, formatLength);    //    currentFormat now contains a modified format string for the current parameter
                amf_string currentFormatMB = currentFormat.c_str();
                //processWidthAndPrecision(currentFormatMB, &p_args[0]);    //    This would extract additional arguments for width and precision and replace * with their values
                for (size_t i = 0; i < currentFormatMB.length(); i++)
                {
                    if (currentFormatMB[i] == '*')
                    {
                        int value = va_arg(p_args, int);
                        char valueString[64];
                        sprintf(valueString, "%d", value);
                        currentFormatMB.replace(i, 1, valueString);
                    }
                }

                switch (*fmt)
                {
                    case 'c':
                    {
                        char ch;
                        switch (*(fmt - 1))
                        {
                            case 'h':
                                ch = static_cast<char>(va_arg(p_args, int));
                                break;
                            case 'l':
                            case 'w':
                                ch = va_arg(p_args, unsigned int);
                                break;
                            default:
                                ch = va_arg(p_args, unsigned int);    //    In a wchar_t version of printf %c means wchar_t
                        }
                        currentArgumentString = ch;
                    }
                        break;
                    case 'C':
                    {
                        char ch;
                        switch (*(fmt - 1))
                        {
                            case 'h':
                                ch = static_cast<char>(va_arg(p_args, int));
                                break;
                            case 'l':
                            case 'w':
                                ch = va_arg(p_args, unsigned int);
                                break;
                            default:
                                ch = static_cast<char>(va_arg(p_args, int));    //    In a wchar_t version of printf %C means char
                        }
                        currentArgumentString = ch;
                    }
                        break;
                    case 's':
                    {
                        const void* str = va_arg(p_args, const void*);
                        if (str != NULL)
                        {
                            switch (*(fmt - 1))
                            {
                                case 'h':
                                    currentArgumentString = reinterpret_cast<const char*>(str);
                                    break;
                                case 'l':
                                case 'w':
                                    currentArgumentString = amf_from_unicode_to_utf8(reinterpret_cast<const wchar_t*>(str));
                                    break;
                                default:
                                    currentArgumentString = reinterpret_cast<const char*>(str);
                            }
                        }
                        else
                        {
                            currentArgumentString = "(null)";
                        }
                    }
                        break;
                    case L'S':
                    {
                        const void* str = va_arg(p_args, const void*);
                        if (str != NULL)
                        {
                            switch (*(fmt - 1))
                            {
                                case 'h':
                                    currentArgumentString = reinterpret_cast<const char*>(str);
                                    break;
                                case 'l':
                                case 'w':
                                    currentArgumentString = amf_from_unicode_to_utf8(reinterpret_cast<const wchar_t*>(str));
                                    break;
                                default:
                                    currentArgumentString = amf_from_unicode_to_utf8(reinterpret_cast<const wchar_t*>(str));
                            }
                        }
                        else
                        {
                            currentArgumentString = "(null)";
                        }
                    }
                        break;
                        //    All integer formats
                    case L'i':
                    case L'd':
                    case L'u':
                    case L'o':
                    case L'x':
                    case L'X':
                    {
                        char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                        switch (*(fmt - 1))
                        {
                            case L'l':
                                if (*(fmt - 1) == L'l')    //    long long
                                {
                                    sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long long));
                                }
                                else
                                {
                                    sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long));
                                }
                                break;
                            case L'h':
                                sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, int));
                                break;
#ifdef _WIN32
                                case L'I':    //    I is Microsoft-specific
#else
                            case L'z':    //    z and t are C99-specific, but seem to be unsupported in VC
                            case L't':
#endif
                                sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, size_t));
                                break;
                            default:
                                sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, int));
                                break;
                        }
                        currentArgumentString = tempBuffer;
                    }
                        break;
                        //    All floating point formats
                    case L'e':
                    case L'E':
                    case L'f':
                    case L'g':
                    case L'G':
                    case L'a':
                    case L'A':
                    {
                        char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                        switch (*(fmt - 1))
                        {
                            case L'l':
                            case L'L':
                                sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, long double));
                                break;
                            default:
                                sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, double));
                                break;
                        }
                        currentArgumentString = tempBuffer;
                    }
                        break;
                        //    Pointer
                    case L'p':
                    {
                        char tempBuffer[64];    //    64 bytes should be enough for any numeric format
                        sprintf(tempBuffer, currentFormatMB.c_str(), va_arg(p_args, const void*));
                        currentArgumentString = tempBuffer;
                    }
                        break;
                    case L'n':
                    {
                        int* dest = va_arg(p_args, int*);
                        *dest = static_cast<int>(totalCount);
                        currentArgumentString.clear();
                    }
                        break;
                }
                size_t length = currentArgumentString.length();
                if (p_outDelegate != NULL)    // If destination buffer is NULL, just count the characters
                {
                    p_outDelegate(p_context, totalCount, currentArgumentString.c_str(), length);
                }
                totalCount += length;
            }    //    if (isOneOf(*fmt, formatSpecifiers))
        }    // if (inFormat)
        else
        {    //    Just copy the character into the output buffer
            if (p_outDelegate != NULL)    // If destination buffer is NULL, just count the characters
            {
                p_outDelegate(p_context, totalCount, fmt, 1);
            }
            ++totalCount;
        }
    }
    return totalCount;
}


typedef struct {
    wchar_t*    m_Buf;
    size_t        m_Size;
} MemBufferContextW;

typedef struct {
    char*    m_Buf;
    size_t        m_Size;
} MemBufferContext;

static size_t writeToMem(void* p_context, size_t p_offset, const wchar_t* p_stringToAdd, size_t p_length)
{
    wchar_t* buf = &(reinterpret_cast<MemBufferContextW*>(p_context)->m_Buf[p_offset]);
    size_t bufSize = reinterpret_cast<MemBufferContextW*>(p_context)->m_Size - 1;    //    -1 to accommodate a trailing '\0'
    for (int i = 0; i < p_length && i < bufSize; i++)
    {
        *buf++ = p_stringToAdd[i];
    }
    return p_length;
}

static size_t writeToFile(void* p_context, size_t, const wchar_t* p_stringToAdd, size_t p_length)
{
    return fwrite(p_stringToAdd, sizeof(wchar_t), p_length, reinterpret_cast<FILE*>(p_context));
}

extern "C"
{
    int vswprintf(wchar_t* p_buf, size_t p_size, const wchar_t* p_fmt, va_list p_args)
    {
        MemBufferContextW context = { p_buf, p_size };
        int bytesWritten = (int)amf_wprintfCore(writeToMem, &context, p_fmt, p_args);
        p_buf[bytesWritten] = L'\0';
        return bytesWritten;
    }

    int wsprintf(wchar_t* p_buf, const wchar_t* p_fmt, ...)
    {
        va_list argptr;
        va_start(argptr, p_fmt);
        return vswprintf(p_buf, static_cast<size_t>(-1), p_fmt, argptr);
    }

    int swprintf(wchar_t* p_buf, size_t p_size, const wchar_t* p_fmt, ...)
    {
        va_list argptr;
        va_start(argptr, p_fmt);
        return vswprintf(p_buf, p_size, p_fmt, argptr);
    }

    int vfwprintf(FILE* p_stream, const wchar_t* p_fmt, va_list p_args)
    {
        return (int)amf_wprintfCore(writeToFile, p_stream, p_fmt, p_args);
    }

    int fwprintf(FILE* p_stream, const wchar_t* p_fmt, ...)
    {
        va_list argptr;
        va_start(argptr, p_fmt);
        return vfwprintf(p_stream, p_fmt, argptr);
    }

#if !defined(__APPLE__)
    int vscwprintf(const wchar_t* p_fmt, va_list p_args)
    {
        return (int)amf_wprintfCore(NULL, NULL, p_fmt, p_args);
    }

    int vscprintf(const char* p_fmt, va_list p_args)
    {
        return (int)amf_printfCore(NULL, NULL, p_fmt, p_args);
    }
#endif
}
#endif

//--------------------------------------------------------------------------------
// Mac doens't have _wcsicmp(0 - poor man implementation
//--------------------------------------------------------------------------------
#ifdef __APPLE__
extern "C"
{
    int _wcsicmp(const wchar_t* s1, const wchar_t* s2)
    {
        amf_wstring low_s1 = s1;
        amf_wstring low_s2 = s2;
        std::transform(low_s1.begin(), low_s1.end(), low_s1.begin(), ::tolower);
        std::transform(low_s2.begin(), low_s2.end(), low_s2.begin(), ::tolower);

        return wcscmp(low_s1.c_str(), low_s2.c_str());
    }
}
#endif
