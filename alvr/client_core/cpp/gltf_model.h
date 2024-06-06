#ifndef ALVRCLIENT_GLTFMODEL_H
#define ALVRCLIENT_GLTFMODEL_H

#include <GLES3/gl3.h>
#include <VrApi_Helpers.h>
#include <VrApi_Types.h>
#include <string>
#include <vector>

#define TINYGLTF_NO_STB_IMAGE_WRITE
#include "tinygltf/tiny_gltf.h"

class GltfModel {
    std::vector<GLuint> m_vbs;
    tinygltf::Model m_model;
    GLuint m_vao;

    int m_position;
    int m_uv;
    GLint m_mMatrix;
    GLint m_mode;

    void drawNodeTree(int node_i, const ovrMatrix4f &transform);
    void drawNode(int node_i, const ovrMatrix4f &transform);
    ovrMatrix4f createNodeTransform(const ovrMatrix4f &baseTransform, const tinygltf::Node &node);

  public:
    void load();
    void drawScene(int position, int uv, GLint mMatrix, GLint mode);
};

#endif // ALVRCLIENT_GLTFMODEL_H
