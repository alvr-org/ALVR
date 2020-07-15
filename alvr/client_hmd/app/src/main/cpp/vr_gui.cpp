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
const float CURSOR_SCALE_FACTOR = 0.05; // -> m at 1 meter of distance
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
            *outCoords = 1.f - *outCoords;
            *outPosition = (1.f - outCoords->x - outCoords->x) * vert3 + outCoords->x * vert2 +
                           outCoords->x * vert1 + offset;
            return true;
        } else {
            return false;
        }
    }
}

VRGUI::ControllerState::ControllerState() : cursorAnimation(Linear, 0.5s) {}

VRGUI::VRGUI() {
    vector<uint8_t> pngData, textureData;
    uint32_t width;
    uint32_t height;

    loadAsset("cursor.png", pngData);
    lodepng::decode(textureData, width, height, pngData);
    mCursorTexture.reset(new Texture(false, width, height, GL_RGBA, textureData));

    loadAsset("pointer_bar_gradient.png", pngData);
    lodepng::decode(textureData, width, height, pngData);
    mPointerBarTexture.reset(new Texture(false, width, height, GL_RGBA, textureData));

    mPointerBarModelTransform = scale(mat4(), {POINTER_BAR_WIDTH, POINTER_BAR_LENGTH, 1});
    mPointerBarModelTransform = translate(mPointerBarModelTransform,
                                          {0, POINTER_BAR_LENGTH / 2, 0});

    for (auto &state : mControllerStates) {
        state.cursorQuad = make_unique<TexturedQuad>(mCursorTexture.get(), mat4());
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

        auto ctrlRotation = toMat4(input.controllersRotation[i]);
        auto transform = ctrlRotation * mPointerBarModelTransform;
        transform = translate(transform, input.controllersPosition[i]);
        // todo: rotate to face headPosition
        controllerState.pointerBarQuad->SetTransform(transform);

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
                                                     &cursorPosition, &dist);
            if (intersection && dist < minDist) {
                minDist = dist;
                cursorPosition = postition;
                cursorCoords = coords;
                closestPanel = panel;
            }
        }

        if (mActivePanel != nullptr && mActivePanel != closestPanel) {
            mActivePanel->SendEvent(InteractionType::CURSOR_LEAVE);
        }

        if (closestPanel != nullptr) {
            controllerState.cursorQuad->SetOpacity(1);

            if (closestPanel != mActivePanel) {
                mActivePanel = closestPanel;
                mActivePanel->SendEvent(InteractionType::CURSOR_ENTER);
            }

            mActivePanel->SendEvent(InteractionType::CURSOR_MOVE, cursorCoords);

            if (!mPrevInput.actionButtonsDown[i] && input.actionButtonsDown[i]) {
                mActivePanel->SendEvent(InteractionType::BUTTON_DOWN);
                controllerState.cursorAnimation.Start(1, CURSOR_PRESSED_SCALE_FACTOR);
            } else if (mPrevInput.actionButtonsDown[i] && !input.actionButtonsDown[i]) {
                mActivePanel->SendEvent(InteractionType::BUTTON_UP);
                controllerState.cursorAnimation.Start(CURSOR_PRESSED_SCALE_FACTOR, 1);
            }

            float scaleValue =
                    controllerState.cursorAnimation.GetValue() * minDist * CURSOR_SCALE_FACTOR;
            auto transform = scale(mat4(), {scaleValue, scaleValue, 1});
            transform = mActivePanel->GetRotation() * transform;
            transform = translate(transform, cursorPosition);
            controllerState.cursorQuad->SetTransform(transform);
        } else {
            controllerState.cursorQuad->SetOpacity(0);
        }
    }
}

void VRGUI::Render(const RenderState &renderState, const mat4 &camera) const {
    for (auto panel : mPanels) {
        panel->Render(renderState, camera);
    }

    for (auto &state : mControllerStates) {
        state.cursorQuad->Render(renderState, camera);
        state.pointerBarQuad->Render(renderState, camera);
    }
}
