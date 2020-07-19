#pragma once

#include "gl_render_utils/textured_quad.h"
#include <glm/glm.hpp>
#include <memory>
#include <functional>

enum class InteractionType {
    CURSOR_ENTER,
    CURSOR_LEAVE,
    CURSOR_HOVER,
    CURSOR_DRAG,
    BUTTON_DOWN,
    BUTTON_UP,
};

class InteractivePanel {
public:
    InteractivePanel(const gl_render_utils::Texture *texture, float width, float height,
                     glm::vec3 position, float yaw, float pitch,
                     std::function<void(InteractionType, glm::vec2)> &interactionCallback);

    glm::mat4 GetWorldTransform() const {
        return mWorldTransform;
    }

    glm::mat4 GetRotation() const {
        return mRotation;
    }

    void SetPoseTransform(const glm::vec3 &position, float yaw, float pitch);

    void SendEvent(InteractionType type, glm::vec2 pos = {}) {
        mInteractionCallback(type, pos);
    }

    void SetOpacity(float opacity) {
        mQuad->SetOpacity(opacity);
    }

    void Render(const gl_render_utils::RenderState &renderState, const glm::mat4 &camera) const;

private:
    glm::mat4 mModelTransform;
    glm::mat4 mRotation;
    glm::mat4 mWorldTransform;

    std::unique_ptr<gl_render_utils::TexturedQuad> mQuad;
    std::function<void(InteractionType, glm::vec2)> mInteractionCallback;
};