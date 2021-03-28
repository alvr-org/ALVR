#pragma once

#include <unordered_map>
#include <vulkan/vulkan.h>
#include "util/fence.h"

namespace wsi
{

class display
{
public:
	static display& get();

	Fence& get_vsync_fence() { return vsync_fence;}
private:
  Fence vsync_fence;
};

}
