#pragma once
#include <windows.h>
#include <mmsystem.h>
#include <mmddk.h>
#include <stdio.h>
#include <tchar.h>
#include "Logger.h"
#include "Settings.h"

/*
 * some good values for block size and count
 */
#define BLOCK_SIZE 1024
#define BLOCK_COUNT 32


/*
Class playing the received microphone samples received from the oculus quest on the CABLE audio device.
Base on the tutorial from:
http://www.planet-source-code.com/vb/scripts/ShowCode.asp?txtCodeId=4422&lngWId=3

Needs the CABLE audio driver https://www.vb-audio.com/Cable/
*/
class MicPlayer
{
public:
	MicPlayer();
	~MicPlayer();
	void playAudio(LPSTR data, int size);
	void waveCallback();

	UINT getMicHWID();
	
	

private:
	/*
 * function prototypes
 */	

	WAVEHDR* allocateBlocks(int size, int count);
	void freeBlocks(WAVEHDR* blockArray);

	

	/*
	 * module level variables
	 */

	WAVEHDR* waveBlocks;
	UINT deviceID = -1;

	int waveCurrentBlock;

	HWAVEOUT hWaveOut; /* device handle */
	WAVEFORMATEX wfx; 
};



