#include "OvrDirectModeComponent.h"
#include "Renderer.h"


OvrDirectModeComponent::OvrDirectModeComponent(std::shared_ptr<Renderer> pVKRender, std::shared_ptr<PoseHistory> poseHistory)
	: m_pVKRender(pVKRender)
	, m_poseHistory(poseHistory)
	, m_submitLayer(0)
{
}

void OvrDirectModeComponent::SetEncoder(std::shared_ptr<CEncoder> pEncoder) {
    m_pEncoder = pEncoder;
}

/** Specific to Oculus compositor support, textures supplied must be created using this method. */
void OvrDirectModeComponent::CreateSwapTextureSet(uint32_t unPid, const SwapTextureSetDesc_t *pSwapTextureSetDesc, SwapTextureSet_t *pOutSwapTextureSet) {

}

/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be used to destroy the entire set. */
void OvrDirectModeComponent::DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) {

}

/** Used to purge all texture sets for a given process. */
void OvrDirectModeComponent::DestroyAllSwapTextureSets(uint32_t unPid) {

}

/** After Present returns, calls this to get the next index to use for rendering. */
void OvrDirectModeComponent::GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2]) {
	(*pIndices)[0]++;
	(*pIndices)[0] %= 3;
	(*pIndices)[1]++;
	(*pIndices)[1] %= 3;
}

/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
void OvrDirectModeComponent::SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2]) {
	m_presentMutex.lock();

	auto pPose = &perEye[0].mHmdPose; // TODO: are both poses the same? Name HMD suggests yes.

	if (m_submitLayer == 0) {
		// Detect FrameIndex of submitted frame by pPose.
		// This is important part to achieve smooth headtracking.
		// We search for history of TrackingInfo and find the TrackingInfo which have nearest matrix value.

		auto pose = m_poseHistory->GetBestPoseMatch(*pPose);
		if (pose) {
			// found the frameIndex
			m_prevTargetTimestampNs = m_targetTimestampNs;
			m_targetTimestampNs = pose->targetTimestampNs;

			m_prevFramePoseRotation = m_framePoseRotation;
			m_framePoseRotation.x = pose->motion.orientation.x;
			m_framePoseRotation.y = pose->motion.orientation.y;
			m_framePoseRotation.z = pose->motion.orientation.z;
			m_framePoseRotation.w = pose->motion.orientation.w;
		}
		else {
			m_targetTimestampNs = 0;
			m_framePoseRotation = HmdQuaternion_Init(0.0, 0.0, 0.0, 0.0);
		}
	}
	if (m_submitLayer < MAX_LAYERS) {
		m_submitLayers[m_submitLayer][0] = perEye[0];
		m_submitLayers[m_submitLayer][1] = perEye[1];
		m_submitLayer++;
	}
	else {
		Warn("Too many layers submitted!\n");
	}

	//CopyTexture();

	m_presentMutex.unlock();
}

/** Submits queued layers for display. */
void OvrDirectModeComponent::Present(vr::SharedTextureHandle_t syncTexture) {

}

void OvrDirectModeComponent::PostPresent(const Throttling_t *pThrottling) {
	WaitForVSync();
}

void OvrDirectModeComponent::CopyTexture(uint32_t layerCount) {

}

bool OvrDirectModeComponent::NewSharedVulkanImage(uint32_t nImageFormat, uint32_t nWidth, uint32_t nHeight, bool bRenderable, bool bMappable, bool bComputeAccess, uint32_t unMipLevels, uint32_t unArrayLayerCount, vr::SharedTextureHandle_t *pSharedHandle) {

}

bool OvrDirectModeComponent::NewSharedVulkanBuffer(size_t nSize, uint32_t nUsageFlags, vr::SharedTextureHandle_t *pSharedHandle) {

}

bool OvrDirectModeComponent::NewSharedVulkanSemaphore(vr::SharedTextureHandle_t *pSharedHandle) {

}