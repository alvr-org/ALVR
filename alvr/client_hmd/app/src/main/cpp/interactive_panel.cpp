#include "interactive_panel.h"

#include <glm/gtc/matrix_transform.hpp>
#include <glm/gtx/euler_angles.hpp>

using namespace std;
using namespace glm;
using namespace gl_render_utils;

InteractivePanel::InteractivePanel(const Texture *texture, float width, float height, vec3 position,
                                   float yaw, float pitch,
                                   function<void(InteractionType, vec2)> &interactionCallback) {
    mModelTransform = scale(mat4(1.f), {width, height, 1});
    mRotation = eulerAngleXY(pitch, yaw);
    mWorldTransform = translate(mRotation * mModelTransform, position);
    mQuad = make_unique<TexturedQuad>(texture, mWorldTransform);
    mInteractionCallback = interactionCallback;
}

void InteractivePanel::SetPoseTransform(const vec3 &position, float yaw, float pitch) {
    mRotation = eulerAngleXY(pitch, yaw);
    mWorldTransform = translate(mRotation * mModelTransform, position);
    mQuad->SetTransform(mWorldTransform);
}

void InteractivePanel::Render(const RenderState &renderState, const mat4 &camera) const {
    mQuad->Render(renderState, camera);
}
