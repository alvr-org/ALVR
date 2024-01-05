# Fixed Foveated Rendering (FFR)

## What is it, why do I need it

In short: The human eye can only see sharp in a very small area (the fovea). That's why we move our eyes constantly to get the feeling that our whole view is a sharp image.
The idea of foveated rendering is, to only render the small portion of the screen where we look at at the highest resolution, and the other parts at a lower resolution. This would massively increase the performance without any noticeable visual impact.
But it has one drawback: You need to track the movement of the eyes. While there are already headsets out there that have eyetracking, the quest does not have it. 

That's why oculus is using Fixed Foveated rendering. There is some research that shows, that if you assume that the user looks at the center of the screen, some parts of the image are more important than others ([Oculus](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-ffr/)). Many games on the Quest use this to improve rendering performance.

## FFR in ALVR

That's not how ALVR is using it :P 

We don't have any influence on how games get rendered, we only get the final rendered image that should be displayed.

What ALVR normally does is: 

- takes that image
- encodes it as a video with the resolution you set at the video tab 
- transmits it to the Quest
- displays the video

With FFR:

- takes the image
- projects the image, keeping the center area at the resolution you set at the video tab, reducing the resolution at the outer regions
- encodes it as a video with the new, lower overall resolution
- transmits the video to the Quest
- reprojects the video to the original size
- displays the image

There are two implementations of FFR by [zarik5](https://github.com/zarik5)

- warp: Uses a radial warping of the video where the center stays the same resolution and the outer regions get "squished" to reduce resolution
- slices: Slices the image into parts (center, left/right, top/bottom) and encodes the outer slices with lower resolution. This method produces a much sharper image.

**Advantages**: Lower resolution results in faster encoding and decoding of the video which will decrease overall latency.  Same bitrate at lower resolution results in higher image quality. On slow networks, bitrate can be reduced resulting in the same quality as without ffr.

**Drawbacks**: Using the warped method results in a slightly overall blurry image. You can compensate this by setting the initial video resolution to 125%.
Slicing will result in a noticeable border where the high res center ends.
Increasing the foveation strength will result in visual artifacts with both methods

## Configuration

That depends on your perception. You should try different settings going from strength = 2 up to 5 for both methods.
The higher you go, the more visual artifacts you will see at the edges of the screen.
For the slice method, you can also set a center offset. This moves the high res center up or down to accommodate games that have more interaction in the upper or lower part of the screen. 

[wikipedia](https://en.wikipedia.org/wiki/Foveated_rendering)
