#include "amf_helper.h"
#include "alvr_server/Logger.h"

#include <dlfcn.h>
#include <fcntl.h>
#include <unistd.h>

namespace alvr
{

class TraceWriter : public amf::AMFTraceWriter
{
public:
    void AMF_CDECL_CALL Write(const wchar_t *, const wchar_t *message) override
    {
        Info("AMF: %ls", message);
    }

    void AMF_CDECL_CALL Flush() override
    {
    }
};

AMFContext::AMFContext()
{
    init();
}

bool AMFContext::isValid() const
{
    return m_valid;
}

amf::AMFFactory *AMFContext::factory() const
{
    return m_factory;
}

amf::AMFContextPtr AMFContext::context() const
{
    return m_context;
}

std::vector<const char*> AMFContext::requiredDeviceExtensions() const
{
    if (!m_context1) {
        return {};
    }
    size_t count;
    m_context1->GetVulkanDeviceExtensions(&count, nullptr);
    std::vector<const char*> out(count);
    m_context1->GetVulkanDeviceExtensions(&count, out.data());
    return out;
}

void AMFContext::initialize(amf::AMFVulkanDevice *dev)
{
    if (!m_context1) {
        throw "No Context1";
    }

    bool ok = m_context1->InitVulkan(dev) == AMF_OK;

    unsetenv("VK_DRIVER_FILES");
    unsetenv("VK_ICD_FILENAMES");

    if (!ok) {
        throw "Failed to initialize Vulkan AMF";
    }
}

const wchar_t *AMFContext::resultString(AMF_RESULT res)
{
    return m_trace->GetResultText(res);
}

AMFContext *AMFContext::get()
{
    static AMFContext *s = nullptr;
    if (!s) {
        s = new AMFContext;
    }
    return s;
}

void AMFContext::init()
{
    void *amf_module = dlopen(AMF_DLL_NAMEA, RTLD_LAZY);
    if (!amf_module) {
        return;
    }

    auto init = (AMFInit_Fn)dlsym(amf_module, AMF_INIT_FUNCTION_NAME);
    if (!init) {
        return;
    }

    if (init(AMF_FULL_VERSION, &m_factory) != AMF_OK) {
        return;
    }

    if (m_factory->GetTrace(&m_trace) != AMF_OK) {
        return;
    }

    m_trace->EnableWriter(AMF_TRACE_WRITER_CONSOLE, false);
    m_trace->EnableWriter(AMF_TRACE_WRITER_DEBUG_OUTPUT, false);
    m_trace->RegisterWriter(L"alvr-amf-trace", new TraceWriter, true);
    m_trace->SetWriterLevel(L"alvr-amf-trace", AMF_TRACE_WARNING);

    if (m_factory->CreateContext(&m_context) != AMF_OK) {
        return;
    }

    m_context1 = amf::AMFContext1Ptr(m_context);

    char *vk_icd_file = getenv("ALVR_AMF_ICD");
    if (!vk_icd_file || access(vk_icd_file, F_OK) != 0) {
        return;
    }

    setenv("VK_DRIVER_FILES", vk_icd_file, 1);
    setenv("VK_ICD_FILENAMES", vk_icd_file, 1);

    m_valid = true;
}

};
