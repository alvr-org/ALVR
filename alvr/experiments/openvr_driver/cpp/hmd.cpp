
#include "hmd.h"
#include <string>

void Hmd::activate_inner() {}

void *Hmd::GetComponent(const char *component_name_and_version) {
    if (std::string(component_name_and_version) == vr::IVRDisplayComponent_Version ||
        (std::string(component_name_and_version) == vr::IVRDriverDirectModeComponent_Version &&
         this->presentation)) {
        return this;
    }
}

void Hmd::GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height) {
    *x = 0;
    *y = 0;
    *width = this->config.preferred_view_width * 2;
    *height = this->config.preferred_view_height;
}

void Hmd::GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height) {
    *width = this->config.preferred_view_width;
    *height = this->config.preferred_view_height;
}

void Hmd::GetEyeOutputViewport(
    vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height) {
    *x = (eye == vr::Eye_Left ? 0 : this->config.preferred_view_width);
    *y = 0;
    *width = this->config.preferred_view_width;
    *height = this->config.preferred_view_height;
}

void Hmd::GetProjectionRaw(vr::EVREye eye, float *left, float *right, float *top, float *bottom) {
    auto fov = this->config.fov[eye];
    *left = fov.left;
    *right = fov.right;
    *top = fov.top;
    *bottom = fov.bottom;
}

vr::DistortionCoordinates_t Hmd::ComputeDistortion(vr::EVREye, float u, float v) {
    return {{u, v}, {u, v}, {u, v}};
}