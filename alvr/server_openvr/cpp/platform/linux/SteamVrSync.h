#pragma once
// Finds the GPU signal that marks the end of SteamVR's write into a shared
// texture, so the encoder reads each texture exactly when it is finished
// instead of after a guessed delay. Linux only.
//
// WHY
// On NVIDIA's Linux driver the dma-buf of a SteamVR texture carries no
// implicit fence, so ALVR could not see when the compositor (or the
// application) finished drawing and slept a fixed time. No fixed time fits
// every load: too short reads the previous frame, too long costs frame rate,
// and with SteamVR's next write overlapping the read one eye shows the next
// frame (seen as the right eye shaking; the compositor draws it last).
// Steam Link's own driver (driver_vrlink.so) waits on a per-texture timeline
// semaphore instead; this file reaches the same semaphore.
//
// UNDOCUMENTED STEAMVR INTERFACES USED HERE
// None of the following is in openvr_driver.h. It was read from Steam Link's
// driver_vrlink.so (SteamVR 2.17.x, October 2026) and checked against the
// live shared memory; the analysis, with addresses, is in the MetaHorizonLinux
// repository, analysis/steamlink-vrlink-driver.md, "Per-texture semaphores".
// A SteamVR update may change any of it; every failure below degrades to
// "no semaphore", and the caller then falls back to the old guard.
//  1. Every shared resource (image or semaphore) has a 0x48-byte "resource
//     state" in POSIX shared memory. Its name is the CRC-32 (zlib, IEEE) of
//     "VR_CompositorResourceState_<handle hex>_0_<namespace>", opened as
//     "/u<uid>-Shm_<crc hex>". In SteamVR's code: CSharedResourceBaseLinux::
//     Import builds "_%lx_%d" and the shared object manager hashes the name.
//  2. <namespace> is vrserver's PID. Steam Link asks vrserver for it over the
//     "SteamVR_Namespace" IPC channel; a driver runs inside vrserver, so
//     getpid() gives the same number. Would break if drivers ever ran in
//     their own process.
//  3. Image state +0x40: the handle of the image's associated timeline
//     semaphore, in the same handle space as vr::SharedTextureHandle_t
//     ((index << 32) | compositor PID; the semaphore's index is the image's
//     + 1). Zero until SteamVR has associated one.
//  4. Semaphore state +0x00: the timeline value of the last write submitted
//     on its image. Read at Present it is the value that write will signal.
//     Steam Link reads it at Present and waits for it on the GPU.
//     Observed limit: for vrcompositor's own textures the right eye's value
//     is often already reached at Present while its write is still to come,
//     so for frames that read them the driver also holds PostPresent until
//     the read (see OvrDirectModeComponent::PostPresent).
// The semaphore fd itself comes through the PUBLIC IVRIPCResourceManagerClient
// (RefResource + ReceiveSharedFd), as an OPAQUE_FD timeline semaphore.
//
// OTHER DEVICES
// Windows: not built there (D3D textures and keyed mutexes instead).
// NVIDIA: tested; the semaphore was present for every texture.
// AMD / Intel (Mesa): not tested. The state comes from SteamVR's
// vendor-neutral code, so it is likely present; if it is not, the caller
// falls back to upstream's guard (implicit fences, which do work on Mesa,
// plus a fixed sleep).
#include "openvr_driver.h"

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <string>
#include <sys/mman.h>
#include <unistd.h>
#include <vector>

namespace steamvrsync {

// UNDOCUMENTED (item 1 and 3 above): size of a resource state and where an
// image's state keeps its semaphore handle.
constexpr size_t StateSize = 0x48;
constexpr size_t ImageSemaphoreOffset = 0x40;

// The CRC-32 SteamVR's shared object manager names its memory with (item 1).
inline uint32_t Crc32(const char* data, size_t length) {
    uint32_t crc = 0xffffffffu;
    for (size_t i = 0; i < length; ++i) {
        crc ^= (uint8_t)data[i];
        for (int bit = 0; bit < 8; ++bit) {
            crc = (crc >> 1) ^ (0xedb88320u & (0u - (crc & 1u)));
        }
    }
    return ~crc;
}

class Sync {
public:
    static Sync& Get() {
        static Sync sync;
        return sync;
    }

    // Handle of the semaphore SteamVR associated with this texture, 0 when
    // there is none (yet). Cached once found. Compositor thread.
    uint64_t SemaphoreOf(uint64_t textureHandle) {
        std::lock_guard<std::mutex> lock(m_mutex);
        auto it = m_semaphoreOf.find(textureHandle);
        if (it != m_semaphoreOf.end()) {
            return it->second;
        }
        const volatile uint8_t* state = State(textureHandle);
        if (!state) {
            return 0;
        }
        uint64_t semaphore
            = __atomic_load_n((const uint64_t*)(state + ImageSemaphoreOffset), __ATOMIC_ACQUIRE);
        if (semaphore != 0) {
            m_semaphoreOf[textureHandle] = semaphore;
        }
        return semaphore;
    }

    // Timeline value of the last write submitted on the semaphore's image;
    // false when its state cannot be mapped. Read at Present.
    bool Value(uint64_t semaphoreHandle, uint64_t* value) {
        std::lock_guard<std::mutex> lock(m_mutex);
        const volatile uint8_t* state = State(semaphoreHandle);
        if (!state) {
            return false;
        }
        // UNDOCUMENTED (item 4): the value sits at the start of the state.
        *value = __atomic_load_n((const uint64_t*)state, __ATOMIC_ACQUIRE);
        return true;
    }

    // A new fd for the semaphore (caller owns it), -1 on failure. The first
    // call takes a reference and receives the fd through SteamVR's IPC
    // resource manager; later calls dup the kept one.
    int DupFd(uint64_t semaphoreHandle) {
        std::lock_guard<std::mutex> lock(m_mutex);
        auto it = m_fds.find(semaphoreHandle);
        if (it == m_fds.end()) {
            auto* mgr = vr::VRIPCResourceManager();
            if (!mgr) {
                return -1;
            }
            int fd = -1;
            uint64_t ipcHandle = 0;
            if (mgr->RefResource(semaphoreHandle, &ipcHandle)) {
                if (mgr->ReceiveSharedFd(ipcHandle, &fd) && fd >= 0) {
                    it = m_fds.emplace(semaphoreHandle, fd).first;
                } else {
                    if (fd >= 0) {
                        close(fd);
                    }
                    mgr->UnrefResource(semaphoreHandle);
                    return -1;
                }
            } else {
                return -1;
            }
        }
        return it->second >= 0 ? dup(it->second) : -1;
    }

    // Drops everything kept for these textures once SteamVR destroyed them:
    // the mapped states, the semaphore's fd and its reference. Returns the
    // semaphores that were associated, so the caller can free its imports.
    std::vector<uint64_t> Forget(std::vector<uint64_t> const& textureHandles) {
        std::lock_guard<std::mutex> lock(m_mutex);
        std::vector<uint64_t> semaphores;
        for (uint64_t texture : textureHandles) {
            auto sem = m_semaphoreOf.find(texture);
            if (sem != m_semaphoreOf.end()) {
                semaphores.push_back(sem->second);
                Unmap(sem->second);
                auto fd = m_fds.find(sem->second);
                if (fd != m_fds.end()) {
                    if (fd->second >= 0) {
                        close(fd->second);
                        auto* mgr = vr::VRIPCResourceManager();
                        if (mgr) {
                            mgr->UnrefResource(sem->second);
                        }
                    }
                    m_fds.erase(fd);
                }
                m_semaphoreOf.erase(sem);
            }
            Unmap(texture);
        }
        return semaphores;
    }

private:
    std::mutex m_mutex;
    std::map<uint64_t, const volatile uint8_t*> m_states;
    std::map<uint64_t, uint64_t> m_semaphoreOf;
    std::map<uint64_t, int> m_fds;

    void Unmap(uint64_t handle) {
        auto it = m_states.find(handle);
        if (it != m_states.end()) {
            munmap((void*)it->second, StateSize);
            m_states.erase(it);
        }
    }

    const volatile uint8_t* State(uint64_t handle) {
        auto it = m_states.find(handle);
        if (it != m_states.end()) {
            return it->second;
        }
        // UNDOCUMENTED (items 1 and 2): the state's name and file. Read only;
        // ALVR never writes SteamVR's state.
        char name[96];
        snprintf(
            name,
            sizeof(name),
            "VR_CompositorResourceState_%llx_0_%u",
            (unsigned long long)handle,
            (unsigned)getpid()
        );
        char path[64];
        snprintf(path, sizeof(path), "/u%u-Shm_%x", (unsigned)getuid(), Crc32(name, strlen(name)));
        int fd = shm_open(path, O_RDONLY, 0);
        if (fd < 0) {
            return nullptr; // not created yet; retried on the next call
        }
        void* map = mmap(nullptr, StateSize, PROT_READ, MAP_SHARED, fd, 0);
        close(fd);
        if (map == MAP_FAILED) {
            return nullptr;
        }
        m_states[handle] = (const volatile uint8_t*)map;
        return (const volatile uint8_t*)map;
    }
};

} // namespace steamvrsync
