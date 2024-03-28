#pragma once
#include "openvr_driver.h"
#include "alvr_server/Utils.h"
#include "CEncoder.h"
#include "Renderer.h"
#include "alvr_server/PoseHistory.h"

#include "alvr_server/Settings.h"

#include <mutex>

class OvrDirectModeComponent : public vr::IVRDriverDirectModeComponent, vr::IVRIPCResourceManagerClient
{
public:
	OvrDirectModeComponent(std::shared_ptr<Renderer> pVKRender, std::shared_ptr<PoseHistory> poseHistory);

	void SetEncoder(std::shared_ptr<CEncoder> pEncoder);

	/** Specific to Oculus compositor support, textures supplied must be created using this method. */
	virtual void CreateSwapTextureSet( uint32_t unPid, const SwapTextureSetDesc_t *pSwapTextureSetDesc, SwapTextureSet_t *pOutSwapTextureSet );

	/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be used to destroy the entire set. */
	virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle);

	/** Used to purge all texture sets for a given process. */
	virtual void DestroyAllSwapTextureSets(uint32_t unPid);

	/** After Present returns, calls this to get the next index to use for rendering. */
	virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2]);

	/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
	* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
	virtual void SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2]);

	/** Submits queued layers for display. */
	virtual void Present(vr::SharedTextureHandle_t syncTexture);
	
	/** Called after Present to allow driver to take more time until vsync after they've successfully acquired the sync texture in Present.*/
	virtual void PostPresent(const Throttling_t *pThrottling);

	void CopyTexture(uint32_t layerCount);

	///IVRIPCResourceManagerClient
	virtual bool NewSharedVulkanImage( uint32_t nImageFormat, uint32_t nWidth, uint32_t nHeight, bool bRenderable, bool bMappable, bool bComputeAccess, uint32_t unMipLevels, uint32_t unArrayLayerCount, vr::SharedTextureHandle_t *pSharedHandle );

	/** Create a new tracked Vulkan Buffer */
	virtual bool NewSharedVulkanBuffer( size_t nSize, uint32_t nUsageFlags, vr::SharedTextureHandle_t *pSharedHandle );

	/** Create a new tracked Vulkan Semaphore */
	virtual bool NewSharedVulkanSemaphore( vr::SharedTextureHandle_t *pSharedHandle );

	/** Grab a reference to hSharedHandle, and optionally generate a new IPC handle if pNewIpcHandle is not nullptr  */
	virtual bool RefResource( vr::SharedTextureHandle_t hSharedHandle, uint64_t *pNewIpcHandle );

	/** Drop a reference to hSharedHandle */
	virtual bool UnrefResource( vr::SharedTextureHandle_t hSharedHandle );

private:
	std::shared_ptr<Renderer> m_pVKRender;
	std::shared_ptr<CEncoder> m_pEncoder;
	std::shared_ptr<PoseHistory> m_poseHistory;

	static const int MAX_LAYERS = 10;
	int m_submitLayer;
	SubmitLayerPerEye_t m_submitLayers[MAX_LAYERS][2];
	vr::HmdQuaternion_t m_prevFramePoseRotation;
	vr::HmdQuaternion_t m_framePoseRotation;
	uint64_t m_targetTimestampNs;
	uint64_t m_prevTargetTimestampNs;

	std::mutex m_presentMutex;
};
