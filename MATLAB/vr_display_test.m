
% Compile native code for communicating with the headset.
mex -I'..\shared' vr_display.cpp

% Launch process to create a window on the hmd.
!..\bin\win32\virtual_display.exe 3200 0 2160 1200 90 1 &

% Load our test image.
A = imread('test.png');

% Swap rows/cols.
A = permute( A, [2 1 3] );

% Transmit to headset.
vr_display( A );
pause

% Sending nothing will exit the separate process.
vr_display();

