//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Hook for MATLAB to push data to virtual_display.exe
//
//==================================================================================================
#include "mex.h"
#include "sharedstate.h"
#include "ipctools.cpp"

CSharedState g_sharedState;
IPCEvent g_newFrame( "RemoteDisplay_NewFrame", false, false );

void transmit( int width, int height, int depth, const unsigned char *pData )
{
	mexPrintf( "vr_display( %d %d %d )\n", width, height, depth );

	if ( !g_sharedState.IsValid() )
		mexErrMsgIdAndTxt( "VALVE:InvalidSharedState", "Unable to open shared memory!" );

	CSharedState::Ptr data( &g_sharedState );
	data->m_nTextureWidth = width;
	data->m_nTextureHeight = height;
	data->m_nTextureFormat = 28; //DXGI_FORMAT_R8G8B8A8_UNORM

	memcpy( data->m_nTextureData, pData, width * height * depth );

	g_newFrame.SetEvent();
}

void shutdown()
{
	CSharedState::Ptr data( &g_sharedState );
    data->m_bShutdown = true;
	g_newFrame.SetEvent();
}

/* The gateway function */
void mexFunction( int nlhs, mxArray *plhs[],
	int nrhs, const mxArray *prhs[] )
{
    /* Calling with no parameters is our signal to shutdown. */
    if ( nrhs == 0 )
    {
        shutdown();
        return;
    }

	if ( nrhs != 1 )
		mexErrMsgIdAndTxt( "VALVE:vr_display:nrhs", "One input required." );
	if ( nlhs != 0 )
		mexErrMsgIdAndTxt( "VALVE:vr_display:nlhs", "Function does not return a value." );
	if ( !mxIsUint8( prhs[ 0 ] ) )
		mexErrMsgIdAndTxt( "VALVE:vr_display:notUint8", "Input must be a matrix of Uint8 values." );

	int w = ( int )mxGetM( prhs[ 0 ] );
	int h = ( int )mxGetN( prhs[ 0 ] );
	const unsigned char *pData = ( const unsigned char * )mxGetData( prhs[ 0 ] );
	transmit( w, h / 4, 4, pData );
}

