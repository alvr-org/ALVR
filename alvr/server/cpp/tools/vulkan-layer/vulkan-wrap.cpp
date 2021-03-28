#include <cstdlib>
#include <cstring>
#include <dlfcn.h>
#include <vector>
#include <vulkan/vulkan.h>

namespace {
struct VkFunctions {
	PFN_vkGetInstanceProcAddr vkGetInstanceProcAddr;
	PFN_vkCreateInstance vkCreateInstance;

	static VkFunctions& instance() {
		static VkFunctions inst;
		return inst;
	}
private:
	VkFunctions() {
		void *module = dlopen("/usr/lib64/libvulkan.so.1", RTLD_NOW | RTLD_LOCAL);
		if (!module) {
			abort();
		}
		vkGetInstanceProcAddr = (PFN_vkGetInstanceProcAddr) dlsym(module, "vkGetInstanceProcAddr");
		vkCreateInstance = (PFN_vkCreateInstance) vkGetInstanceProcAddr(NULL, "vkCreateInstance");
	}
};
}

extern "C" {

VKAPI_ATTR VkResult VKAPI_CALL override_vkCreateInstance(
    const VkInstanceCreateInfo*                 pCreateInfo,
    const VkAllocationCallbacks*                pAllocator,
    VkInstance*                                 pInstance)
{
	VkInstanceCreateInfo override_pCreateInfo = *pCreateInfo;
	std::vector<const char*> extensions(pCreateInfo->ppEnabledExtensionNames, pCreateInfo->ppEnabledExtensionNames + pCreateInfo->enabledExtensionCount);
	extensions.push_back(VK_EXT_HEADLESS_SURFACE_EXTENSION_NAME);
	override_pCreateInfo.ppEnabledExtensionNames = extensions.data();
	override_pCreateInfo.enabledExtensionCount = extensions.size();
	auto res = VkFunctions::instance().vkCreateInstance(&override_pCreateInfo, pAllocator, pInstance);
	return res;
}

PFN_vkVoidFunction vkGetInstanceProcAddr(
    VkInstance                                  instance,
    const char*                                 pName) {
  // Override vkCreateInstance in order to add required extensions
	if (strcmp(pName, "vkCreateInstance") == 0)
	{
		return (PFN_vkVoidFunction)override_vkCreateInstance;
	}
	return VkFunctions::instance().vkGetInstanceProcAddr(instance, pName);
}
}
