#include <dlfcn.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <xf86drmMode.h>

#include <fstream>
#include <filesystem>

#define PICOJSON_USE_INT64
#include "../server/cpp/alvr_server/include/picojson.h"

#define LOAD_FN(f) \
    if (!real_##f) { \
        real_##f = reinterpret_cast<decltype(real_##f)>(dlsym(RTLD_NEXT, #f)); \
        if (!real_##f) { \
            ERR("Failed to load %s", #f); \
            abort(); \
        } \
    } \

#define LOG(f, ...) printf(f "\n" __VA_OPT__(,) __VA_ARGS__)
#define ERR(f, ...) fprintf(stderr, f "\n" __VA_OPT__(,) __VA_ARGS__)

template <typename X, typename Y>
static constexpr bool compare_ptr(X x, Y y)
{
    return reinterpret_cast<void*>(x) == reinterpret_cast<void*>(y);
}

struct wl_registry_listener {
    void (*global)(void *data, struct wl_registry *wl_registry, uint32_t name, const char *interface, uint32_t version);
    void (*global_remove)(void *data, struct wl_registry *wl_registry, uint32_t name);
};

struct wp_drm_lease_device_v1_listener {
    void (*drm_fd)(void *data, struct wp_drm_lease_device_v1 *wp_drm_lease_device_v1, int32_t fd);
    void (*connector)(void *data, struct wp_drm_lease_device_v1 *wp_drm_lease_device_v1, struct wp_drm_lease_connector_v1 *id);
    void (*done)(void *data, struct wp_drm_lease_device_v1 *wp_drm_lease_device_v1);
    void (*released)(void *data, struct wp_drm_lease_device_v1 *wp_drm_lease_device_v1);
};

struct wp_drm_lease_connector_v1_listener {
    void (*name)(void *data, struct wp_drm_lease_connector_v1 *wp_drm_lease_connector_v1, const char *name);
    void (*description)(void *data, struct wp_drm_lease_connector_v1 *wp_drm_lease_connector_v1, const char *description);
    void (*connector_id)(void *data, struct wp_drm_lease_connector_v1 *wp_drm_lease_connector_v1, uint32_t connector_id);
    void (*done)(void *data, struct wp_drm_lease_connector_v1 *wp_drm_lease_connector_v1);
    void (*withdrawn)(void *data, struct wp_drm_lease_connector_v1 *wp_drm_lease_connector_v1);
};

struct wp_drm_lease_v1_listener {
    void (*lease_fd)(void *data, struct wp_drm_lease_v1 *wp_drm_lease_v1, int32_t leased_fd);
    void (*finished)(void *data, struct wp_drm_lease_v1 *wp_drm_lease_v1);
};

static struct wp_drm_lease_device_v1 {} fake_device_id;
static struct wp_drm_lease_connector_v1 {} fake_connector_id;
static struct wp_drm_lease_request_v1 {} fake_lease_request_id;
static struct wp_drm_lease_v1 {} fake_lease_id;

static int drm_fd = -1;
static int drm_connector_id = -1;

static void open_drm_fd()
{
    static drmModeResPtr (*real_drmModeGetResources)(int fd) = nullptr;
    LOAD_FN(drmModeGetResources);
    for(auto cardCandidate : std::filesystem::directory_iterator("/dev/dri")) {
        if(cardCandidate.path().filename().string().rfind("card", 0) == 0) {
            LOG("cardCandidateFound: file=%s", cardCandidate.path().c_str());
            drm_fd = open(cardCandidate.path().c_str(), O_RDONLY);
            auto res = real_drmModeGetResources(drm_fd);
            if (res && res->count_connectors) {
                drm_connector_id = res->connectors[0];
                break;
            }
        }
    }
    LOG("DRM: fd=%d, connector_id=%d", drm_fd, drm_connector_id);
}

static int (*real_wl_proxy_add_listener)(struct wl_proxy *proxy, void (**implementation)(void), void *data);
static int hooked_wl_proxy_add_listener(struct wl_proxy *proxy, void (**implementation)(void), void *data)
{
    // wp_drm_lease_connector_v1
    if (compare_ptr(proxy, &fake_connector_id)) {
        LOG("LISTENER wp_drm_lease_connector_v1");
        auto listener = reinterpret_cast<struct wp_drm_lease_connector_v1_listener*>(implementation);
        listener->name(data, &fake_connector_id, "ALVR_name");
        listener->description(data, &fake_connector_id, "ALVR_description");
        listener->connector_id(data, &fake_connector_id, drm_connector_id);
        listener->done(data, &fake_connector_id);
        LOG("LISTENER done");
        return 0;
    }

    // wp_drm_lease_v1
    if (compare_ptr(proxy, &fake_lease_id)) {
        LOG("LISTENER wp_drm_lease_v1");
        auto listener = reinterpret_cast<struct wp_drm_lease_v1_listener*>(implementation);
        listener->lease_fd(data, &fake_lease_id, drm_fd);
        LOG("LISTENER done");
        return 0;
    }

    // wp_drm_lease_device_v1
    if (compare_ptr(proxy, &fake_device_id)) {
        LOG("LISTENER wp_drm_lease_device_v1");
        auto listener = reinterpret_cast<struct wp_drm_lease_device_v1_listener*>(implementation);
        open_drm_fd();
        listener->drm_fd(data, &fake_device_id, drm_fd);
        if (drm_connector_id != -1) {
            listener->connector(data, &fake_device_id, &fake_connector_id);
        }
        listener->done(data, &fake_device_id);
        LOG("LISTENER done");
        return 0;
    }

    const char *name = *(*reinterpret_cast<const char***>(proxy));

    if (strcmp(name, "wl_registry") == 0) {
        LOG("LISTENER wl_registry");
        auto listener = reinterpret_cast<struct wl_registry_listener*>(implementation);
        listener->global(data, reinterpret_cast<struct wl_registry*>(proxy), 0, "wp_drm_lease_device_v1", 1);
        LOG("LISTENER done");
        return 0;
    }

    return real_wl_proxy_add_listener(proxy, implementation, data);
}

static struct wl_proxy *(*real_wl_proxy_marshal_flags)(struct wl_proxy *proxy, uint32_t opcode, const struct wl_interface *interface, uint32_t version, uint32_t flags, ...);
static struct wl_proxy *hooked_wl_proxy_marshal_flags(struct wl_proxy *proxy, uint32_t opcode, const struct wl_interface *interface, uint32_t version, uint32_t flags, ...)
{
    // wp_drm_lease_connector_v1
    if (compare_ptr(proxy, &fake_connector_id)) {
        if (opcode == 0) {
            LOG("CALL wp_drm_lease_connector_v1_destroy");
        } else {
            ERR("Unknown wp_drm_lease_connector_v1 opcode=%u", opcode);
        }
        return nullptr;
    }

    // wp_drm_lease_request_v1
    if (compare_ptr(proxy, &fake_lease_request_id)) {
        if (opcode == 0) {
            LOG("CALL wp_drm_lease_request_v1_request_connector");
        } else if (opcode == 1) {
            LOG("CALL wp_drm_lease_request_v1_submit");
            return reinterpret_cast<struct wl_proxy*>(&fake_lease_id);
        } else {
            ERR("Unknown wp_drm_lease_request_v1 opcode=%u", opcode);
        }
        return nullptr;
    }

    // wp_drm_lease_device_v1
    if (compare_ptr(proxy, &fake_device_id)) {
        if (opcode == 0) {
            LOG("CALL wp_drm_lease_device_v1_create_lease_request");
            return reinterpret_cast<struct wl_proxy*>(&fake_lease_request_id);
        } else if (opcode == 1) {
            LOG("CALL wp_drm_lease_device_v1_release");
        } else {
            ERR("Unknown wp_drm_lease_device_v1 opcode=%u", opcode);
        }
        return nullptr;
    }

    const char *name = **reinterpret_cast<const char***>(proxy);
    const char *iname = *reinterpret_cast<const char**>(const_cast<struct wl_interface*>(interface));

    if (strcmp(name, "wl_registry") == 0 && strcmp(iname, "wp_drm_lease_device_v1") == 0 && opcode == 0) {
        LOG("CALL wl_registry_bind - wp_drm_lease_device_v1");
        return reinterpret_cast<struct wl_proxy*>(&fake_device_id);
    }

    __builtin_return(__builtin_apply(reinterpret_cast<void(*)(...)>(real_wl_proxy_marshal_flags), __builtin_apply_args(), 1024));
}

extern "C" void *SDL_LoadFunction(void *handle, const char *name)
{
    static void *(*real_SDL_LoadFunction)(void *handle, const char *name) = nullptr;
    LOAD_FN(SDL_LoadFunction);

#define HOOK(f) \
    if (strcmp(name, #f) == 0) { \
        LOG("HOOK %s", #f); \
        real_##f = reinterpret_cast<decltype(real_##f)>(real_SDL_LoadFunction(handle, #f)); \
        return reinterpret_cast<void*>(hooked_##f); \
    } \

    HOOK(wl_proxy_add_listener);
    HOOK(wl_proxy_marshal_flags);

#undef HOOK

    return real_SDL_LoadFunction(handle, name);
}

extern "C" drmModeConnectorPtr drmModeGetConnector(int fd, uint32_t connectorId)
{
    LOG("CALL drmModeGetConnector(%d, %u)", fd, connectorId);

    static drmModeConnectorPtr (*real_drmModeGetConnector)(int fd, uint32_t connectorId) = nullptr;
    LOAD_FN(drmModeGetConnector);

    auto con = real_drmModeGetConnector(fd, connectorId);
    if (con) {
        auto sessionFile = std::ifstream(getenv("ALVR_SESSION_JSON"));
        auto json = std::string(std::istreambuf_iterator<char>(sessionFile), std::istreambuf_iterator<char>());
        picojson::value v;
        picojson::parse(v, json);
        auto config = v.get("openvr_config");

        con->count_modes = 1;
        con->modes = (drmModeModeInfo*)calloc(1, sizeof(drmModeModeInfo));
        con->modes->hdisplay = config.get("eye_resolution_width").get<int64_t>() * 2;
        con->modes->vdisplay = config.get("eye_resolution_height").get<int64_t>();
    }
    return con;
}

__attribute__((constructor)) static void lib_init()
{
    LOG("ALVR: drm-lease shim loaded");

    unsetenv("LD_PRELOAD");
}
