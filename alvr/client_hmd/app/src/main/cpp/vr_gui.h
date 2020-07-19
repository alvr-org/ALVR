#pragma once

#include <vector>
#include <memory>
#include <glm/gtc/quaternion.hpp>

#include "interactive_panel.h"
#include "animation_curve.h"

struct GUIInput {
    glm::vec3 headPosition = {};
    glm::vec3 controllersPosition[2] = {};
    glm::quat controllersRotation[2] = {};
    bool actionButtonsDown[2] = {}; // trigger, A or X; left (0) right (1) controller
};

class VRGUI {
public:
    // requires a GL context and initialized asset manager
    VRGUI();

    void AddPanel(InteractivePanel *panel) {
        mPanels.push_back(panel);
    }

    void RemovePanel(const InteractivePanel *panel);

    void Update(const GUIInput &input);

    void Render(const gl_render_utils::RenderState &renderState, const glm::mat4 &camera) const;

private:
    GUIInput mPrevInput = {};
    int mActiveControllerIdx = -1;

    std::vector<InteractivePanel *> mPanels;
    InteractivePanel *mActivePanel = nullptr;

    std::unique_ptr<gl_render_utils::Texture> mCursorIdleTexture;
    std::unique_ptr<gl_render_utils::Texture> mCursorPressTexture;
    std::unique_ptr<gl_render_utils::Texture> mPointerBarTexture;
    glm::mat4 mPointerBarModelTransform;

    struct ControllerState {
        std::unique_ptr<gl_render_utils::TexturedQuad> cursorIdleQuad;
        std::unique_ptr<gl_render_utils::TexturedQuad> cursorPressQuad;
        std::unique_ptr<gl_render_utils::TexturedQuad> pointerBarQuad;
    } mControllerStates[2];
};