/*
 * Copyright (c) 2017-2020 Arm Limited.
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
 * @file swapchain_base.hpp
 *
 * @brief Contains the class definition for a base swapchain.
 */

#pragma once

#include <pthread.h>
#include <semaphore.h>
#include <thread>
#include <vulkan/vulkan.h>

#include <layer/private_data.hpp>
#include <util/custom_allocator.hpp>
#include <util/timed_semaphore.hpp>
#include <util/pose.hpp>

namespace wsi {
struct swapchain_image {
    enum status {
        INVALID,
        ACQUIRED,
        PENDING,
        PRESENTED,
        FREE,
    };

    /* Implementation specific data */
    void *data{nullptr};

    VkImage image{VK_NULL_HANDLE};
    status status{swapchain_image::INVALID};

    VkFence present_fence{VK_NULL_HANDLE};
    VkSemaphore semaphore{VK_NULL_HANDLE};
    uint64_t semaphore_value = 0;

    TrackedDevicePose_t pose;
};

/**
 * @brief Base swapchain class
 *
 * - the swapchain implementations inherit from this class.
 * - the VkSwapchain will hold a pointer to this class.
 * - much of the swapchain implementation is done by this class, as the only things needed
 *   in the implementation are how to create a presentable image and how to present an image.
 */
class swapchain_base {
  public:
    swapchain_base(layer::device_private_data &dev_data, const VkAllocationCallbacks *allocator);

    virtual ~swapchain_base() { /* nop */
    }

    /**
     * @brief Create swapchain.
     *
     * Perform all swapchain initialization, create presentable images etc.
     */
    VkResult init(VkDevice device, const VkSwapchainCreateInfoKHR *swapchain_create_info);

    /**
     * @brief Acquires a free image.
     *
     * Current implementation blocks until a free image is available.
     *
     * @param timeout Unused since we block until a free image is available.
     *
     * @param semaphore A semaphore signaled once an image is acquired.
     *
     * @param fence A fence signaled once an image is acquired.
     *
     * @param pImageIndex The index of the acquired image.
     *
     * @return VK_SUCCESS on completion.
     */
    VkResult acquire_next_image(uint64_t timeout, VkSemaphore semaphore, VkFence fence,
                                uint32_t *image_index);

    /**
     * @brief Gets the number of swapchain images or a number of at most
     * m_num_swapchain_images images.
     *
     * @param pSwapchainImageCount Used to return number of images in
     * the swapchain if second parameter is nullptr or represents the
     * number of images to be returned in second parameter.
     *
     * @param pSwapchainImage Array of VkImage handles.
     *
     * @return If number of requested images is less than the number of available
     * images in the swapchain returns VK_INCOMPLETE otherwise VK_SUCCESS.
     */
    VkResult get_swapchain_images(uint32_t *swapchain_image_count, VkImage *swapchain_image);

    /**
     * @brief Submits a present request for the supplied image.
     *
     * @param queue The queue to which the submission will be made to.
     *
     * @param pPresentInfo Information about the swapchain and image to be presented.
     *
     * @param imageIndex The index of the image to be presented.
     *
     * @return If queue submission fails returns error of vkQueueSubmit, if the
     * swapchain has a descendant who started presenting returns VK_ERROR_OUT_OF_DATE_KHR,
     * otherwise returns VK_SUCCESS.
     */
    VkResult queue_present(VkQueue queue, const VkPresentInfoKHR *present_info,
                           const uint32_t image_index);

  protected:
    layer::device_private_data &m_device_data;

    /**
     * @brief Handle to the page flip thread.
     */
    std::thread m_page_flip_thread;

    /**
     * @brief Whether the page flip thread has to continue running or terminate.
     */
    bool m_page_flip_thread_run;

    /**
     * @brief In case we encounter threading or drm errors we need a way to
     * notify the user of the failure. When this flag is false, acquire_next_image
     * will return an error code.
     */
    bool m_is_valid;

    struct ring_buffer {
        /* Ring buffer to hold the image indexes. */
        uint32_t *ring;
        /* Head of the ring. */
        uint32_t head;
        /* End of the ring. */
        uint32_t tail;
        /* Size of the ring. */
        uint32_t size;
    };
    /**
     * @brief A semaphore to be signalled once a page flip event occurs.
     */
    util::timed_semaphore m_page_flip_semaphore;

    /**
     * @brief A semaphore to be signalled once the swapchain has one frame on screen.
     */
    sem_t m_start_present_semaphore;

    /**
     * @brief Defines if the pthread_t and sem_t members of the class are defined.
     *
     * As they are opaque types theer's no known invalid value that we ca initialize to,
     * and therefore determine if we need to cleanup.
     */
    bool m_thread_sem_defined;

    /**
     * @brief A flag to track if it is the first present for the chain.
     */
    bool m_first_present;

    /**
     * @brief In order to present the images in a FIFO order we implement
     * a ring buffer to hold the images queued for presentation. Since the
     * two pointers (head and tail) are used by different
     * threads and we do not allow the application to acquire more images
     * than we have we eliminate race conditions.
     */
    ring_buffer m_pending_buffer_pool;

    /**
     * @brief User provided memory allocation callbacks.
     */
    const util::allocator m_allocator;

    /**
     * @brief Vector of images in the swapchain.
     */
    util::vector<swapchain_image> m_swapchain_images;

    /**
     * @brief Handle to the surface object this swapchain will present images to.
     */
    VkSurfaceKHR m_surface;

    /**
     * @brief present mode to use for this swapchain
     */
    VkPresentModeKHR m_present_mode;

    /**
     * @brief Descendant of this swapchain.
     * Used to check whether or not a descendant of this swapchain has started
     * presenting images to the surface already. If it has, any calls to queuePresent
     * for this swapchain will return VK_ERROR_OUT_OF_DATE_KHR.
     */
    VkSwapchainKHR m_descendant;

    /**
     * @brief Ancestor of this swapchain.
     * Used to check whether the ancestor swapchain has completed all of its
     * pending page flips (this is required before this swapchain presents for the
     * first time.
     */
    VkSwapchainKHR m_ancestor;

    /**
     *  @brief Handle to the logical device the swapchain is created for.
     */
    VkDevice m_device;

    /**
     *  @brief Handle to the queue used for signalling submissions
     */
    VkQueue m_queue;

    /**
     * @brief Return the VkAllocationCallbacks passed in this object constructor.
     */
    const VkAllocationCallbacks *get_allocation_callbacks() {
        return m_allocator.get_original_callbacks();
    }

    /**
     * @brief Method to wait on all pending buffers to be displayed.
     */
    void wait_for_pending_buffers();

    /**
     * @brief Remove cached ancestor.
     */
    void clear_ancestor();

    /**
     * @brief Remove cached descendant.
     */
    void clear_descendant();

    /**
     * @brief Deprecate this swapchain.
     *
     * If an application replaces an old swapchain with a new one, the older swapchain
     * needs to be deprecated. This method releases all the FREE images and sets the
     * descendant of the swapchain. We do not need to care about images in other states
     * at this point since they will be released by the page flip thread.
     *
     * @param descendant Handle to the descendant swapchain.
     */
    void deprecate(VkSwapchainKHR descendant);

    /**
     * @brief Platform specific initialization
     */
    virtual VkResult init_platform(VkDevice device,
                                   const VkSwapchainCreateInfoKHR *swapchain_create_info) = 0;

    /**
     * @brief Base swapchain teardown.
     *
     * Even though the inheritance gives us a nice way to defer display specific allocation
     * and presentation outside of the base class, it however robs the children classes - which
     * also happen to do some of their state setting - the oppurtunity to do the last clean up
     * call, as the base class' destructor is called at the end. This method provides a way to do
     * it. The destructor is a virtual function and much of the swapchain teardown happens in this
     * method which gets called from the child's destructor.
     */
    void teardown();

    /**
     * @brief Creates a new swapchain image.
     *
     * @param image_create_info Data to be used to create the image.
     *
     * @param image Handle to the image.
     *
     * @return If image creation is successful returns VK_SUCCESS, otherwise
     * will return VK_ERROR_OUT_OF_DEVICE_MEMORY or VK_ERROR_INITIALIZATION_FAILED
     * depending on the error that occured.
     */
    virtual VkResult create_image(const VkImageCreateInfo &image_create_info,
                                  swapchain_image &image) = 0;


    virtual void submit_image(uint32_t pending_index) = 0;

    /**
     * @brief Method to present and image
     *
     * @param pending_index Index of the pending image to be presented.
     *
     */
    virtual void present_image(uint32_t pending_index) = 0;

    /**
     * @brief Transition a presented image to free.
     *
     * Called by swapchain implementation when a new image has been presented.
     *
     * @param presented_index Index of the image to be marked as free.
     */
    void unpresent_image(uint32_t presented_index);

    /**
     * @brief Method to release a swapchain image
     *
     * @param image Handle to the image about to be released.
     */
    virtual void destroy_image(swapchain_image &image){};

    /**
     * @brief Hook for any actions to free up a buffer for acquire
     *
     * @param[in,out] timeout time to wait, in nanoseconds. 0 doesn't block,
     *                        UINT64_MAX waits indefinately. The timeout should
     *                        be updated if a sleep is required - this can
     *                        be set to 0 if the semaphore is now not expected
     *                        block.
     */
    virtual VkResult get_free_buffer(uint64_t *timeout) { return VK_SUCCESS; }

  private:
    /**
     * @brief Wait for a buffer to become free.
     */
    VkResult wait_for_free_buffer(uint64_t timeout);

    /**
     * @brief A semaphore to be signalled once a free image becomes available.
     *
     * Uses a custom semaphore implementation that uses a condition variable.
     * it is slower, but has a safe timedwait implementation.
     *
     * This is kept private as waiting should be done via wait_for_free_buffer().
     */
    util::timed_semaphore m_free_image_semaphore;

    /**
     * @brief Per swapchain thread function that handles page flipping.
     *
     * This thread should be running for the lifetime of the swapchain.
     * The thread simply calls the implementation's present_image() method.
     * There are 3 main cases we cover here:
     *
     * 1. On the first present of the swapchain if the swapchain has
     *    an ancestor we must wait for it to finish presenting.
     * 2. The normal use case where we do page flipping, in this
     *    case change the currently PRESENTED image with the oldest
     *    PENDING image.
     * 3. If the enqueued image is marked as FREE it means the
     *    descendant of the swapchain has started presenting so we
     *    should release the image and continue.
     *
     * The function always waits on the page_flip_semaphore of the
     * swapchain. Once it passes that we must wait for the fence of the
     * oldest pending image to be signalled, this means that the gpu has
     * finished rendering to it and we can present it. From there on the
     * logic splits into the above 3 cases and if an image has been
     * presented then the old one is marked as FREE and the free_image
     * semaphore of the swapchain will be posted.
     **/
    void page_flip_thread();

    uint32_t m_last_acquired_image = 0;

    std::vector<VkFence> m_fences;
};

} /* namespace wsi */
