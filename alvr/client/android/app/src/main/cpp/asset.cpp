#include "asset.h"

AAssetManager *g_assetManager = NULL;
jobject g_javaAssetManager = NULL;

//
// File system operations for loading gltf model.
//

bool AssetFileExists(const std::string &abs_filename, void *) {
    AAsset *asset = AAssetManager_open(g_assetManager, abs_filename.c_str(), AASSET_MODE_STREAMING);
    if (asset == NULL) {
        return false;
    }
    AAsset_close(asset);
    return true;
}

std::string AssetExpandFilePath(const std::string &path, void *) {
    return path;
}

bool AssetReadWholeFile(std::vector<unsigned char> *out,
                               std::string *err, const std::string &path,
                               void *) {
    AAsset *asset = AAssetManager_open(g_assetManager, path.c_str(), AASSET_MODE_STREAMING);
    if (asset == NULL) {
        return false;
    }

    int length = AAsset_getLength(asset);
    out->resize(length);

    if (AAsset_read(asset, &(*out)[0], length) != length) {
        out->resize(0);
        AAsset_close(asset);
        return false;
    }

    AAsset_close(asset);
    return true;
}

bool AssetWriteWholeFile(std::string *err, const std::string &filepath,
                        const std::vector<unsigned char> &contents, void *) {
    return false;
}

tinygltf::FsCallbacks gAssetFsCallbacks {.FileExists=AssetFileExists,
        .ExpandFilePath=AssetExpandFilePath,
        .ReadWholeFile=AssetReadWholeFile,
        .WriteWholeFile=AssetWriteWholeFile};



void setAssetManager(JNIEnv *env, jobject assetManager) {
    if (g_assetManager == NULL) {
        g_javaAssetManager = env->NewGlobalRef(assetManager);
        g_assetManager = AAssetManager_fromJava(env, g_javaAssetManager);
    }
}

bool loadAsset(const char *path, std::vector<unsigned char> &buffer) {
    AAsset *asset = AAssetManager_open(g_assetManager, path, AASSET_MODE_STREAMING);
    if (asset == NULL) {
        return false;
    }

    int length = AAsset_getLength(asset);
    buffer.resize(length);

    if (AAsset_read(asset, &buffer[0], length) != length) {
        AAsset_close(asset);
        return false;
    }

    AAsset_close(asset);
    return true;
}