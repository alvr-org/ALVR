#pragma once

#include "render_pipeline.h"
#include <glm/glm.hpp>

namespace gl_render_utils {

    // Textured quad in 3D. Without transformations, the quad is a square of side 1 centered at the origin.
    class TexturedQuad {
    public:
        TexturedQuad(const Texture *texture, glm::mat4 transform);

        void SetTransform(const glm::mat4 &transform) {
            mTransform = transform;
        }

        void SetOpacity(float opacity) {
            mOpacity = opacity;
        }

        void Render(const RenderState &renderState, const glm::mat4 &camera) const;

    private:
        struct UniformBlock {
            glm::mat4 mvp;
            float opacity;
        };

        std::unique_ptr<RenderPipeline> mPipeline;
        glm::mat4 mTransform;
        float mOpacity = 1;
    };

}

