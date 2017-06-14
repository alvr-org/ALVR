
% Compile native code for communicating with the headset.
mex -I'..\shared' vr_display.cpp

% Launch process to create a window on the hmd.
!..\bin\win32\virtual_display.exe 3200 0 2160 1200 90 1 &

% Load our test image.
A = imread('test.png');
B=A;

B = permute( B, [2 1 3] );
B = cat( 3, B, 255 * ones( size( B, 1 ), size( B, 2 ), 'uint8' ) );

w=size(A,2);
h=size(A,1);
n=h/4;

pixr={};
for i=1:4,
    Rimg=reshape(B(:,n*(i-1)+1:n*i,1),w*n,1);
    Gimg=reshape(B(:,n*(i-1)+1:n*i,2),w*n,1);
    Bimg=reshape(B(:,n*(i-1)+1:n*i,3),w*n,1);
    Aimg=reshape(B(:,n*(i-1)+1:n*i,4),w*n,1);
    pixels=[Rimg,Gimg,Bimg,Aimg]';
    pixr{i}=reshape(pixels(:,1:n*w),w,h);
end
    
B(:,:,1)=pixr{1};
B(:,:,2)=pixr{2};
B(:,:,3)=pixr{3};
B(:,:,4)=pixr{4};

% Transmit to headset.
vr_display( B );
pause

% Sending nothing will exit the separate process.
vr_display();

