#pragma once

#include "../win32/amf/public/common/AMFFactory.h"
#include "../win32/amf/public/include/core/VulkanAMF.h"

namespace alvr
{

class AMFContext
{
public:
    bool isValid() const;
    amf::AMFFactory *factory() const;
    amf::AMFContextPtr context() const;
    std::vector<const char*> requiredDeviceExtensions() const;
    void initialize(amf::AMFVulkanDevice *dev);

    static AMFContext *get();

private:
    explicit AMFContext();

    void init();

    bool m_valid = false;
    amf::AMFFactory *m_factory = nullptr;
    amf::AMFContextPtr m_context;
    amf::AMFContext1Ptr m_context1;
};

};
