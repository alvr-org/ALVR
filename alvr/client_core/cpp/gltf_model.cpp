#include "gltf_model.h"

#define TINYGLTF_IMPLEMENTATION
#define STB_IMAGE_IMPLEMENTATION
#define TINYGLTF_NO_STB_IMAGE_WRITE

#include "tinygltf/tiny_gltf.h"
#include "utils.h"

const unsigned char *LOBBY_ROOM_GLTF_PTR;
unsigned int LOBBY_ROOM_GLTF_LEN;
const unsigned char *LOBBY_ROOM_BIN_PTR;
unsigned int LOBBY_ROOM_BIN_LEN;

bool AssetFileExists(const std::string &abs_filename, void *) {
    return true;
}

std::string AssetExpandFilePath(const std::string &path, void *) {
    return path;
}

bool AssetReadWholeFile(std::vector<unsigned char> *out,
                               std::string *, const std::string &path,
                               void *) {
    out->resize(LOBBY_ROOM_BIN_LEN);
    memcpy(&(*out)[0], LOBBY_ROOM_BIN_PTR, LOBBY_ROOM_BIN_LEN);

    return true;
}

bool AssetWriteWholeFile(std::string *, const std::string &,
                        const std::vector<unsigned char> &, void *) {
    return false;
}

tinygltf::FsCallbacks gAssetFsCallbacks {.FileExists=AssetFileExists,
        .ExpandFilePath=AssetExpandFilePath,
        .ReadWholeFile=AssetReadWholeFile,
        .WriteWholeFile=AssetWriteWholeFile};

void GltfModel::load() {
    tinygltf::TinyGLTF loader;
    std::string err, warn;

    loader.SetFsCallbacks(gAssetFsCallbacks);

    auto buffer = std::vector<unsigned char>(LOBBY_ROOM_GLTF_LEN);
    memcpy(&buffer[0], LOBBY_ROOM_GLTF_PTR, LOBBY_ROOM_GLTF_LEN);
    bool ret = loader.LoadASCIIFromString(&m_model, &err, &warn, (char *) &buffer[0], buffer.size(), "");

    LOGI("GltfModel loaded. ret=%d scenes=%lu defaultScene=%d err=%s.\nwarn=%s", ret, m_model.scenes.size(), m_model.defaultScene, err.c_str(), warn.c_str());

    m_vbs.resize(m_model.bufferViews.size());

    GL(glGenVertexArrays(1, &m_vao));
    GL(glBindVertexArray(m_vao));

    for (auto accessor : m_model.accessors) {
        auto bufferView = m_model.bufferViews[accessor.bufferView];
        auto buffer = m_model.buffers[bufferView.buffer];

        glGenBuffers(1, &m_vbs[accessor.bufferView]);
        glBindBuffer(bufferView.target, m_vbs[accessor.bufferView]);
        glBufferData(bufferView.target, bufferView.byteLength,
                     &buffer.data.at(0) + bufferView.byteOffset, GL_STATIC_DRAW);
        glBindBuffer(bufferView.target, 0);
    }

    GL(glBindVertexArray(0));
}

void GltfModel::drawScene(int position, int uv,
                          int normal, GLint color, GLint mMatrix, GLint mode) {
    if(m_model.scenes.size() == 0) {
        return;
    }
    auto &scene = m_model.scenes[m_model.defaultScene];

    m_position = position;
    m_uv = uv;
    m_normal = normal;
    m_color = color;
    m_mMatrix = mMatrix;
    m_mode = mode;

    GL(glBindVertexArray(m_vao));

    ovrMatrix4f transform = ovrMatrix4f_CreateIdentity();

    for (auto node_i : scene.nodes) {
        drawNodeTree(node_i, transform);
    }

    GL(glBindVertexArray(0));
}

void GltfModel::drawNodeTree(int node_i, const ovrMatrix4f &transform) {
    auto &node = m_model.nodes[node_i];

    ovrMatrix4f nodeTransform = createNodeTransform(transform, node);

    drawNode(node_i, nodeTransform);

    for (auto &child_i : node.children) {
        drawNodeTree(child_i, nodeTransform);
    }
}

void GltfModel::drawNode(int node_i, const ovrMatrix4f &transform) {
    auto &node = m_model.nodes[node_i];
    if (node.mesh < 0) {
        return;
    }
    auto &mesh = m_model.meshes[node.mesh];

    for (auto &prim : mesh.primitives) {
        if (prim.indices < 0) {
            continue;
        }
        for (auto &att : prim.attributes) {
            const std::string &name = att.first;
            int att_i = att.second;
            auto &accessor = m_model.accessors[att_i];
            auto &bufferView = m_model.bufferViews[accessor.bufferView];

            glBindBuffer(bufferView.target, m_vbs[accessor.bufferView]);

            int size = 1;
            if (accessor.type == TINYGLTF_TYPE_SCALAR) {
                size = 1;
            } else if (accessor.type == TINYGLTF_TYPE_VEC2) {
                size = 2;
            } else if (accessor.type == TINYGLTF_TYPE_VEC3) {
                size = 3;
            } else if (accessor.type == TINYGLTF_TYPE_VEC4) {
                size = 4;
            } else {
                LOGE("accessor.type is invalid. type=%d", accessor.type);
            }

            int index = -1;
            if (name == "POSITION") {
                index = m_position;
            } else if (name == "TEXCOORD_0") {
                index = m_uv;
            } else if (name == "NORMAL") {
                index = m_normal;
            }
            if (index != -1) {
                // Compute byteStride from Accessor + BufferView combination.
                int byteStride = accessor.ByteStride(
                        m_model.bufferViews[accessor.bufferView]);
                GL(glVertexAttribPointer(index, size,
                                         accessor.componentType,
                                         accessor.normalized ? GL_TRUE : GL_FALSE,
                                         byteStride,
                                         (void *) accessor.byteOffset));
                GL(glEnableVertexAttribArray(index));
            }
        }

        auto &material = m_model.materials[prim.material];

        tinygltf::ColorValue colorValue = {1.0, 1.0, 1.0, 1.0};

        auto it = material.values.find("baseColorFactor");
        if (it != material.values.end()) {
            colorValue = it->second.ColorFactor();
        }
        GL(glUniform4f(m_color, colorValue[0], colorValue[1], colorValue[2], colorValue[3]));

        GL(glUniformMatrix4fv(m_mMatrix, 1, true, (float *) &transform));

        if(material.name == "Plane") {
            GL(glUniform1i(m_mode, 0));
        }else if(material.name == "Message") {
            GL(glUniform1i(m_mode, 1));
        }else{
            GL(glUniform1i(m_mode, 2));
        }

        auto &indexAccessor = m_model.accessors[prim.indices];
        auto &bufferView = m_model.bufferViews[indexAccessor.bufferView];

        GL(glBindBuffer(bufferView.target, m_vbs[indexAccessor.bufferView]));

        int mode = -1;
        if (prim.mode == TINYGLTF_MODE_TRIANGLES) {
            mode = GL_TRIANGLES;
        } else if (prim.mode == TINYGLTF_MODE_TRIANGLE_STRIP) {
            mode = GL_TRIANGLE_STRIP;
        } else if (prim.mode == TINYGLTF_MODE_TRIANGLE_FAN) {
            mode = GL_TRIANGLE_FAN;
        } else if (prim.mode == TINYGLTF_MODE_POINTS) {
            mode = GL_POINTS;
        } else if (prim.mode == TINYGLTF_MODE_LINE) {
            mode = GL_LINES;
        } else if (prim.mode == TINYGLTF_MODE_LINE_LOOP) {
            mode = GL_LINE_LOOP;
        } else {
            LOGE("Unknown primitive mode. mode=%d", prim.mode);
            continue;
        }

        GL(glDrawElements(mode, indexAccessor.count, indexAccessor.componentType,
                          (void *) indexAccessor.byteOffset));
        GL(glDisableVertexAttribArray(m_position));
    }
}

ovrMatrix4f
GltfModel::createNodeTransform(const ovrMatrix4f &baseTransform, const tinygltf::Node &node) {
    ovrMatrix4f nodeTransform = baseTransform;
    if(node.translation.size() == 3) {
        ovrMatrix4f translation = ovrMatrix4f_CreateTranslation(node.translation[0], node.translation[1], node.translation[2]);
        nodeTransform = ovrMatrix4f_Multiply(&nodeTransform, &translation);
    }
    if(node.rotation.size() == 4) {
        ovrQuatf q;
        q.x = node.rotation[0];
        q.y = node.rotation[1];
        q.z = node.rotation[2];
        q.w = node.rotation[3];
        ovrMatrix4f rotation = ovrMatrix4f_CreateFromQuaternion(&q);
        nodeTransform = ovrMatrix4f_Multiply(&nodeTransform, &rotation);
    }
    if(node.scale.size() == 3) {
        ovrMatrix4f scale = ovrMatrix4f_CreateScale(node.scale[0], node.scale[1], node.scale[2]);
        nodeTransform = ovrMatrix4f_Multiply(&nodeTransform, &scale);
    }
    return nodeTransform;
}
