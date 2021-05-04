/*
 * Copyright (c) 2017, 2019 Arm Limited.
 *
 * SPDX-License-Identifier: MIT
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#include <cassert>
#include <cerrno>

#include "timed_semaphore.hpp"

namespace util {

VkResult timed_semaphore::init(unsigned count) {
    int res;

    m_count = count;

    pthread_condattr_t attr;
    res = pthread_condattr_init(&attr);
    /* the only failure that can occur is ENOMEM */
    assert(res == 0 || res == ENOMEM);
    if (res != 0) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }

    res = pthread_condattr_setclock(&attr, CLOCK_MONOTONIC);
    /* only programming error can cause _setclock to fail */
    assert(res == 0);

    res = pthread_cond_init(&m_cond, &attr);
    /* the only failure that can occur that is not programming error is ENOMEM */
    assert(res == 0 || res == ENOMEM);
    if (res != 0) {
        pthread_condattr_destroy(&attr);
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }

    res = pthread_condattr_destroy(&attr);
    /* only programming error can cause _destroy to fail */
    assert(res == 0);

    res = pthread_mutex_init(&m_mutex, NULL);
    /* only programming errors can result in failure */
    assert(res == 0);

    initialized = true;

    return VK_SUCCESS;
}

timed_semaphore::~timed_semaphore() {
    int res;
    (void)res; /* unused when NDEBUG */

    if (initialized) {
        res = pthread_cond_destroy(&m_cond);
        assert(res == 0); /* only programming error (EBUSY, EINVAL) */

        res = pthread_mutex_destroy(&m_mutex);
        assert(res == 0); /* only programming error (EBUSY, EINVAL) */
    }
}

VkResult timed_semaphore::wait(uint64_t timeout) {
    VkResult retval = VK_SUCCESS;
    int res;

    assert(initialized);

    res = pthread_mutex_lock(&m_mutex);
    assert(res == 0); /* only fails with programming error (EINVAL) */

    if (m_count == 0) {
        switch (timeout) {
        case 0:
            retval = VK_NOT_READY;
            break;
        case UINT64_MAX:
            res = pthread_cond_wait(&m_cond, &m_mutex);
            assert(res == 0); /* only fails with programming error (EINVAL) */

            break;
        default:
            struct timespec diff = {/* narrowing casts */
                                    static_cast<time_t>(timeout / (1000 * 1000 * 1000)),
                                    static_cast<long>(timeout % (1000 * 1000 * 1000))};

            struct timespec now;
            res = clock_gettime(CLOCK_MONOTONIC, &now);
            assert(res == 0); /* only fails with programming error (EINVAL, EFAULT, EPERM) */

            /* add diff to now, handling overflow */
            struct timespec end = {now.tv_sec + diff.tv_sec, now.tv_nsec + diff.tv_nsec};

            if (end.tv_nsec >= 1000 * 1000 * 1000) {
                end.tv_nsec -= 1000 * 1000 * 1000;
                end.tv_sec++;
            }

            res = pthread_cond_timedwait(&m_cond, &m_mutex, &end);
            /* only fails with programming error, other than timeout */
            assert(res == 0 || res == ETIMEDOUT);
            if (res != 0) {
                retval = VK_TIMEOUT;
            }
        }
    }
    if (retval == VK_SUCCESS) {
        assert(m_count > 0);
        m_count--;
    }
    res = pthread_mutex_unlock(&m_mutex);
    assert(res == 0); /* only fails with programming error (EPERM) */

    return retval;
}

void timed_semaphore::post() {
    int res;
    (void)res; /* unused when NDEBUG */

    assert(initialized);

    res = pthread_mutex_lock(&m_mutex);
    assert(res == 0); /* only fails with programming error (EINVAL) */

    m_count++;

    res = pthread_cond_signal(&m_cond);
    assert(res == 0); /* only fails with programming error (EINVAL) */

    res = pthread_mutex_unlock(&m_mutex);
    assert(res == 0); /* only fails with programming error (EPERM) */
}

} /* namespace util */
