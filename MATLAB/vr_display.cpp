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
	//mexPrintf( "vr_display( %d %d %d )\n", width, height, depth );

	if ( !g_sharedState.IsValid() )
		mexErrMsgIdAndTxt( "VALVE:InvalidSharedState", "Unable to open shared memory!" );

	CSharedState::Ptr data( &g_sharedState );
	data->m_nTextureWidth = width;
	data->m_nTextureHeight = height;
	data->m_nTextureFormat = 28; //DXGI_FORMAT_R8G8B8A8_UNORM

	//memcpy( data->m_nTextureData, pData, width * height * depth );

	// Matlab stores color planes separately.
	const unsigned char *pInR = pData;
	const unsigned char *pInG = pInR + width * height;
	const unsigned char *pInB = pInG + width * height;

	// Interleave RGBA data to proper DXGI format.
	unsigned char *pOut = data->m_nTextureData;
	switch ( depth )
	{
	case 1:
		for ( int i = 0; i < width * height; i++ )
		{
			*pOut++ = *pInR++;
			*pOut++ = 0xFF;
			*pOut++ = 0xFF;
			*pOut++ = 0xFF;
		}
		break;
	case 2:
		for ( int i = 0; i < width * height; i++ )
		{
			*pOut++ = *pInR++;
			*pOut++ = *pInG++;
			*pOut++ = 0xFF;
			*pOut++ = 0xFF;
		}
		break;
	default:
		for ( int i = 0; i < width * height; i++ )
		{
			*pOut++ = *pInR++;
			*pOut++ = *pInG++;
			*pOut++ = *pInB++;
			*pOut++ = 0xFF;
		}
		break;
	}

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

	if ( nrhs > 2 )
		mexErrMsgIdAndTxt( "VALVE:vr_display:nrhs", "One or two inputs required." );
	if ( nlhs != 0 )
		mexErrMsgIdAndTxt( "VALVE:vr_display:nlhs", "Function does not return a value." );
	if ( !mxIsUint8( prhs[ 0 ] ) )
		mexErrMsgIdAndTxt( "VALVE:vr_display:notUint8", "Input must be a matrix of Uint8 values." );

	int M = ( int )mxGetM( prhs[ 0 ] );
	int N = ( int )mxGetN( prhs[ 0 ] );

	/* Assume RGB data if not specified. */
	int depth = 3;
	if ( nrhs == 2 )
	{
		depth = ( int )*mxGetPr( prhs[ 1 ] );
		if ( depth < 1 || depth > 4 )
			mexErrMsgIdAndTxt( "VALVE:vr_display:invalidDepth", "Depth must be in range [1..4]." );
	}

	const unsigned char *pData = ( const unsigned char * )mxGetData( prhs[ 0 ] );
	transmit( M, N / depth, depth, pData );
}

