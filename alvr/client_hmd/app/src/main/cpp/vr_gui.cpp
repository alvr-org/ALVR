#include "vr_gui.h"

#include <glm/gtc/matrix_transform.hpp>
#include <glm/gtx/intersect.hpp>
#include <glm/gtx/quaternion.hpp>
#include <lodepng/lodepng.h>
#include "asset.h"
#include "utils.h"

using namespace std;
using namespace glm;
using namespace gl_render_utils;

const float POINTER_BAR_WIDTH = 0.01;
const float POINTER_BAR_LENGTH = 0.3;
const float CURSOR_SCALE_BASE = 0.01;
const float CURSOR_SCALE_FACTOR = 0.015; // -> m at 1 meter of distance
const float CURSOR_OFFSET = 0.001;
const float CURSOR_PRESSED_SCALE_FACTOR = 0.7;

// outCoord: x, y normalized coordinates to be used as input for the interactive panel
// outPosition: position of the visual representation of the cursor
// outDistance: controller-cursor distance to be used for choosing the active panel
bool cursorPositionOnQuad(const vec3 &controllerPosition, const vec3 &direction,
                          const mat4 &quadWorldTransform, vec2 *outCoords, vec3 *outPosition,
                          float *outDistance) {
    // 0 - 1
    // | / |
    // 2 - 3

    vec3 vert0 = quadWorldTransform * vec4(-0.5, 0.5, 0, 1);
    vec3 vert1 = quadWorldTransform * vec4(0.5, 0.5, 0, 1);
    vec3 vert2 = quadWorldTransform * vec4(-0.5, -0.5, 0, 1);
    vec3 vert3 = quadWorldTransform * vec4(0.5, -0.5, 0, 1);

    // lift the cursor off the quad plane to avoid z fighting
    vec3 offset = normalize(cross(vert2 - vert0, vert1 - vert0)) * CURSOR_OFFSET;

    bool intersection = intersectRayTriangle(controllerPosition, direction, vert0, vert1, vert2,
                                             *outCoords, *outDistance);
    if (intersection) {
        *outPosition = (1.f - outCoords->x - outCoords->y) * vert0 + outCoords->x * vert1 +
                       outCoords->y * vert2 + offset;
        return true;
    } else {
        intersection = intersectRayTriangle(controllerPosition, direction, vert3, vert2, vert1,
                                            *outCoords, *outDistance);
        if (intersection) {
            *outPosition = (1.f - outCoords->x - outCoords->y) * vert3 + outCoords->x * vert2 +
                           outCoords->y * vert1 + offset;
            *outCoords = 1.f - *outCoords;
            return true;
        } else {
            return false;
        }
    }
}

VRGUI::VRGUI() {
    uint32_t width;
    uint32_t height;

    vector<uint8_t> pngData, textureData;
    loadAsset("cursor_idle.png", pngData);
    lodepng::decode(textureData, width, height, pngData);
    mCursorIdleTexture = make_unique<Texture>(false, width, height, GL_RGBA, textureData);

    pngData.clear();
    textureData.clear();
    loadAsset("cursor_press.png", pngData);
    lodepng::decode(textureData, width, height, pngData);
    mCursorPressTexture = make_unique<Texture>(false, width, height, GL_RGBA, textureData);

    pngData.clear();
    textureData.clear();
    loadAsset("pointer_bar_gradient.png", pngData);
    lodepng::decode(textureData, width, height, pngData);
    mPointerBarTexture = make_unique<Texture>(false, width, height, GL_RGBA, textureData);

    auto scaling = scale(mat4(1.f), {POINTER_BAR_WIDTH, POINTER_BAR_LENGTH, 1});
    auto rotation = rotate(mat4(1.f), (float) -M_PI_2, {1, 0, 0});
    auto translation = translate(mat4(1.f), {0, 0, -POINTER_BAR_LENGTH / 2});
    mPointerBarModelTransform = translation * rotation * scaling;

    for (auto &state : mControllerStates) {
        state.cursorIdleQuad = make_unique<TexturedQuad>(mCursorIdleTexture.get(), mat4(1.f));
        state.cursorPressQuad = make_unique<TexturedQuad>(mCursorPressTexture.get(), mat4(1.f));
        state.pointerBarQuad = make_unique<TexturedQuad>(mPointerBarTexture.get(),
                                                         mPointerBarModelTransform);
    }
}

void VRGUI::RemovePanel(const InteractivePanel *panel) {
    auto panelIt = std::find(mPanels.begin(), mPanels.end(), panel);
    if (panelIt != mPanels.end()) {
        mPanels.erase(panelIt);
    }
    if (mActivePanel == panel) {
        mActivePanel = nullptr;
    }
}

void VRGUI::Update(const GUIInput &input) {
    for (int i = 0; i < 2; i++) {
        auto &controllerState = mControllerStates[i];

        // Pointer bar

        auto ctrlRotation = toMat4(input.controllersRotation[i]);
        auto ctrlTranslation = translate(mat4(1.f), input.controllersPosition[i]);
        auto ctrlTransform = ctrlTranslation * ctrlRotation * mPointerBarModelTransform;
        // todo: rotate to face headPosition
        controllerState.pointerBarQuad->SetTransform(ctrlTransform);

        // Cursor position and appearance

        controllerState.cursorIdleQuad->SetOpacity(0);
        controllerState.cursorIdleQuad->SetTransform(translate(mat4(1.f), {0, 0, 1}));
        controllerState.cursorPressQuad->SetOpacity(0);
        controllerState.cursorPressQuad->SetTransform(translate(mat4(1.f), {0, 0, 1}));

        auto direction = ctrlRotation * vec4(0, 0, -1, 0);

        vec3 cursorPosition;
        vec2 cursorCoords;
        float minDist = FLT_MAX;
        InteractivePanel *closestPanel = nullptr;
        for (auto panel : mPanels) {
            vec3 postition;
            vec2 coords;
            float dist;
            bool intersection = cursorPositionOnQuad(input.controllersPosition[i], direction,
                                                     panel->GetWorldTransform(), &coords,
                                                     &postition, &dist);
            if (intersection && dist < minDist) {
                minDist = dist;
                cursorPosition = postition;
                cursorCoords = coords;
                closestPanel = panel;
            }
        }

        if (closestPanel != nullptr) {
            auto cursorQuad = input.actionButtonsDown[i] ? controllerState.cursorPressQuad.get()
                                                         : controllerState.cursorIdleQuad.get();

            float headCursorDist = distance(input.headPosition, cursorPosition);
            float scaleValue = headCursorDist * CURSOR_SCALE_FACTOR + CURSOR_SCALE_BASE;
            auto scaling = scale(mat4(1.f), {scaleValue, scaleValue, 1});
            auto rotation = closestPanel->GetRotation();
            auto translation = translate(mat4(1.f), cursorPosition);
            cursorQuad->SetTransform(translation * rotation * scaling);
            cursorQuad->SetOpacity(1);
        }

        // Interaction

        if (i == mActiveControllerIdx || input.actionButtonsDown[i]) {
            mActiveControllerIdx = i;

            if (mActivePanel != nullptr && mActivePanel != closestPanel) {
                mActivePanel->SendEvent(InteractionType::CURSOR_LEAVE, cursorCoords);
                mActivePanel = nullptr;
            }

            if (closestPanel != nullptr) {
                if (closestPanel != mActivePanel) {
                    mActivePanel = closestPanel;
                    mActivePanel->SendEvent(InteractionType::CURSOR_ENTER, cursorCoords);
                }

                if (!mPrevInput.actionButtonsDown[i] && input.actionButtonsDown[i]) {
                    mActivePanel->SendEvent(InteractionType::BUTTON_DOWN, cursorCoords);
                } else if (mPrevInput.actionButtonsDown[i] && !input.actionButtonsDown[i]) {
                    mActivePanel->SendEvent(InteractionType::BUTTON_UP, cursorCoords);
                } else if (mPrevInput.actionButtonsDown[i] && input.actionButtonsDown[i]) {
                    mActivePanel->SendEvent(InteractionType::CURSOR_DRAG, cursorCoords);
                } else {
                    mActivePanel->SendEvent(InteractionType::CURSOR_HOVER, cursorCoords);
                }
            }
        }
    }

    mPrevInput = input;
}

void VRGUI::Render(const RenderState &renderState, const mat4 &camera) const {
    for (auto panel : mPanels) {
        panel->Render(renderState, camera);
    }

    for (auto &state : mControllerStates) {
        state.cursorIdleQuad->Render(renderState, camera);
        state.cursorPressQuad->Render(renderState, camera);
        state.pointerBarQuad->Render(renderState, camera);
    }
}
