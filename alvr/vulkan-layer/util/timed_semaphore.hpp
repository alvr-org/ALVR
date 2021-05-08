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

/**
 * @file timed_semaphore.hpp
 *
 * @brief Contains the class definition for a semaphore with a relative timedwait
 *
 * sem_timedwait takes an absolute time, based on CLOCK_REALTIME. Simply
 * taking the current time and adding on a relative timeout is not correct,
 * as the system time may change, resulting in an incorrect timeout period
 * (potentially by a significant amount).
 *
 * We therefore have to re-engineer semaphores using condition variables.
 *
 * This code does not use the C++ standard library to avoid exceptions.
 */

#pragma once

extern "C" {
#include <pthread.h>
}

#include <vulkan/vulkan.h>

namespace util {

/**
 * brief semaphore with a safe relative timed wait
 *
 * sem_timedwait takes an absolute time, based on CLOCK_REALTIME. Simply
 * taking the current time and adding on a relative timeout is not correct,
 * as the system time may change, resulting in an incorrect timeout period
 * (potentially by a significant amount).
 *
 * We therefore have to re-engineer semaphores using condition variables.
 *
 * This code does not use the C++ standard library to avoid exceptions.
 */
class timed_semaphore {
  public:
    /* copying not implemented */
    timed_semaphore &operator=(const timed_semaphore &) = delete;
    timed_semaphore(const timed_semaphore &) = delete;

    ~timed_semaphore();
    timed_semaphore() : initialized(false){};

    /**
     * @brief initializes the semaphore
     *
     * @param count initial value of the semaphore
     * @retval VK_ERROR_OUT_OF_HOST_MEMORY out of memory condition from pthread calls
     * @retval VK_SUCCESS on success
     */
    VkResult init(unsigned count);

    /**
     * @brief decrement semaphore, waiting (with timeout) if the value is 0
     *
     * @param timeout time to wait (ns). 0 doesn't block, UINT64_MAX waits indefinately.
     * @retval VK_TIMEOUT timeout was non-zero and reached the timeout
     * @retval VK_NOT_READY timeout was zero and count is 0
     * @retval VK_SUCCESS on success
     */
    VkResult wait(uint64_t timeout);

    /**
     * @brief increment semaphore, potentially unblocking a waiting thread
     */
    void post();

  private:
    /**
     * @brief true if the semaphore has been initialized
     *
     * Determines if the destructor should cleanup the mutex and cond.
     */
    bool initialized;
    /**
     * @brief semaphore value
     */
    unsigned m_count;

    pthread_mutex_t m_mutex;
    pthread_cond_t m_cond;
};

} /* namespace util */
