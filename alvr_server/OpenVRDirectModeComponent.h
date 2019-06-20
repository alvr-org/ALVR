//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#pragma once

#include <map>

#include <openvr_driver.h>

#include "packet_types.h"
#include "openvr-utils/d3drender.h"
#include "Listener.h"
#include "FrameEncoder.h"
#include "RecenterManager.h"

class OpenVRDirectModeComponent : public vr::IVRDriverDirectModeComponent
{
public:
	OpenVRDirectModeComponent(std::shared_ptr<CD3DRender> pD3DRender,
		std::shared_ptr<FrameEncoder> pEncoder,
		std::shared_ptr<Listener> Listener,
		std::shared_ptr<RecenterManager> recenterManager);

	void OnPoseUpdated(TrackingInfo &info);

	/** Specific to Oculus compositor support, textures supplied must be created using this method. */
	virtual void CreateSwapTextureSet(uint32_t unPid, const SwapTextureSetDesc_t *pSwapTextureSetDesc, vr::SharedTextureHandle_t(*pSharedTextureHandles)[3]) override;

	/** Used to textures created using CreateSwapTextureSet.  Only one of the set's handles needs to be used to destroy the entire set. */
	virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t sharedTextureHandle) override;

	/** Used to purge all texture sets for a given process. */
	virtual void DestroyAllSwapTextureSets(uint32_t unPid) override;

	/** After Present returns, calls this to get the next index to use for rendering. */
	virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t sharedTextureHandles[2], uint32_t(*pIndices)[2]) override;

	/** Call once per layer to draw for this frame.  One shared texture handle per eye.  Textures must be created
	* using CreateSwapTextureSet and should be alternated per frame.  Call Present once all layers have been submitted. */
	virtual void SubmitLayer(const SubmitLayerPerEye_t(&perEye)[2], const vr::HmdMatrix34_t *pPose) override;

	/** Submits queued layers for display. */
	virtual void Present(vr::SharedTextureHandle_t syncTexture) override;

	void CopyTexture(uint32_t layerCount);

private:
	std::shared_ptr<CD3DRender> mD3DRender;
	std::shared_ptr<FrameEncoder> mEncoder;
	std::shared_ptr<Listener> mListener;
	std::shared_ptr<RecenterManager> mRecenterManager;

	// Resource for each process
	struct ProcessResource {
		ComPtr<ID3D11Texture2D> textures[3];
		HANDLE sharedHandles[3];
		uint32_t pid;
	};
	std::map<HANDLE, std::pair<ProcessResource *, int> > m_handleMap;

	static const int MAX_LAYERS = 10;
	int mSubmitLayer;
	SubmitLayerPerEye_t mSubmitLayers[MAX_LAYERS][2];
	vr::HmdQuaternion_t mPrevFramePoseRotation;
	vr::HmdQuaternion_t mFramePoseRotation;
	uint64_t mSubmitFrameIndex;
	uint64_t mSubmitClientTime;
	uint64_t mPrevSubmitFrameIndex;
	uint64_t mPrevSubmitClientTime;

	uint64_t mLastReferencedFrameIndex;
	uint64_t mLastReferencedClientTime;

	IPCMutex mPoseMutex;
	struct TrackingHistoryFrame {
		TrackingInfo info;
		vr::HmdMatrix34_t rotationMatrix;
	};
	std::list<TrackingHistoryFrame> mPoseBuffer;
};