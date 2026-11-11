# Controller emulation

The goal of this task is to integrate controller emulation into the ALVR emulator so we can provide synthetic controller input.

Requirements:

- The user can toggle controller emulation for each controller (left and right) independently.
- The user can emulate controller input using the mouse (see below)
- The user can provide 6DoF controller pose (position and orientation) for each controller independently.
- The user can emulate input of all buttons and axes of the controller, including triggers, thumbsticks, and buttons as already handled by ALVR.
- The user can select the controller to emulate, this will restrict the set of emulated inputs.
- The user can show or hide the emulated controller in the VR environment.

## User interface

### Toolbar

The header toolbar will get a second inputs row: 

```
Inputs | Controller [L | R] [Quest v] [Display] [Reset]
``` 

- Where [L | R] is a toggle for the left or right controller, 
- [Quest v] is a dropdown to select the controller type to emulate
- [Display] toggles the visibility of the controller
- [Reset] resets the controller state to the default state (position, orientation, buttons and axes).

### Positioning

The controllers positions are defined in a head attached coordinate system, where the origin is at the center of the head, the Z axis points up, the Y axis points forward, and the X axis points to the right. They will move and rotate with the head. (The coordinate system used on the wire might be different, however the above will allow carrying the controllers around.)

> **Important**: These axes will be used in this design document, however they can be adjusted to match the ALVR backend coordinate system for consistency in the implementation.

The starting position is in front of the user, below the centre of the camera, and slightly to the left or right depending on the controller. The exact starting position can be configured in file based settings (no need for GUI).

### Visualization

The controllers are always displayed as 2D icons over the 3D view at their projected position. If the projected position is outside the view frustum, the icon will be displayed at the edge of the view frustum, pointing in the direction of the controller.

If the controller visibility is turned on the controller will be displayed in the 3D view as a 3D model, with accurate position, orientation and size. The model will be provided in a GLTF file and attached to the controller profile in the settings (see below). The model will be loaded at runtime, so the user can add new models without recompiling the code.

### Mouse input

#### 6-DoF movement and rotation

To provide 6-DoF movement, we will split up the movement and rotation into 2D + 1D inputs:

- For movement, we will provide a 2D input for the X and Z axes (movement on a vertical plane), and a 1D input for the Y axis (distance of that plane from head).
- For rotation, we will provide a 2D input for the yaw and pitch axes, and a 1D input for the roll axis.

As the user approaches the controller icon with the mouse the mouse input panel for the specific controller will appear below the controller icon:

```
 /---------+----------+---------+--------\
/    ^     |     ^    |  /---\  |    />   \
|  <-+->   |    /     | v  +  v | <--+--> |
|    v     |   v      |         |    \>   |
|  Planar  | Forward  |         | Pitch / |
\ movement | movement |  Roll   |  Yaw    /
 \---------+----------+---------+--------/
```

The user can drag on the buttons to perform the translation and rotation:

- For movement the amount of translation is the same as the amount of drag.
- For rotation the amount of rotation is proportional to the amount of drag, with a configurable sensitivity.

Velocity and other derived quantities can be calculated from the position and orientation changes over time and reported properly.

#### Buttons and axes

When the controllers are enabled their skeumorphic representation will be displayed in the left and right bottom corners of the screen, even when the controller is not visible in the 3D view (the user could still interact with them with their hand even if offscreen):

```
  \---------/  \
  |   LT    | \ \
 /-----------\ \ \
/  /---\      \ | |
|  | L |  +-+ |
|  \---/  |Y| |
|         +-+ /
| +-+ +-+    /--/
| |=| |X| TR |LG|
| +-+ +-+    |--/
\------------/
```

The controller panel will mimic the layout of the actual controller: above a left controller can be seen with the left thumbstick (L), the left trigger (LT), the left grip (LG), the buttons (X, Y), the thumbrest / trackpad (TR) and the menu button (=). The buttons can be clicked on. The thumbstick can be dragged to provide analog input. The grip and trigger can be either clicked on by the left button to provide a digital input, or dragged with the right mouse button to provide analog input.

Middle clicking the thumbstick will simulate pressing the thumbstick button.

The trackpad can be dragged on with the left button to provide analog input.

The force (force inputs) feedback of the controller will be visualized by showing vibration indicators: e.g at the top right / left corner of the controller panel as shown above (controller vibration). Or having the TR button flash when the trackpad is being vibrated. The amplitude of the vibration will be visualized by the brightness of the indicator, while frequency can be indicated by the speed of subtle blinking.

Ideally the analog controls should reproduce the analog movement: e.g. the thumbstick can be translated by a 2D vector, while the trigger and grip can be translated by a 1D vector. The visualization should fully reflect the endpoints of the input (e.g. fully pressed trigger).

Button touches are emulated by right clicking on the button, which will highlight the button to indicate that it is being touched.

> **Important**: While the input approach works like above, the actual input should be consistent, e.g. the user adding trigger input will also provide the trigger touch input, and the user adding thumbstick input will also provide the thumbstick touch input.

Some of the controls might be absent based on the controller type selected. For example, the Quest controller does not have a trackpad, so the trackpad will be hidden when emulating a Quest controller. The controller profiles should be defined as user editable text based settings, with the profile name, id, and the list of supported controls and any other controller specific settings. The controller profiles should be loaded at runtime, so the user can add new profiles without recompiling the code (as long it can be fully satisfied with the available controls).

The basic settings file should come with all ALVR supported controllers predefined, but the user can add new profiles or modify existing ones. The controller profiles should be documented in the settings documentation.

> **Note**: the controller profiles emulate how ALVR would handle that controller in real-life. E.g. if ALVR remaps some inputs or emulates a different controller based on the actual controller input, this emulation should not worry about that, it should just behave the exact same way as ALVR would handle that controller in real-life.

## Web API

The web API of the emulator will be extended to provide access to all the controller emulation features. The API will provide endpoints to:

- Enable / disable controller emulation for each controller independently
- Set the controller type to emulate for each controller independently
- Set the controller position and orientation for each controller independently
- Set the controller button and axis states for each controller independently, the states are held until the next update, but can be reset using the API or by the user interface. (convenience click methods to perform a button press / release as one action will be provided) Reset can be done using the reset button, or by interacting with the given input from the user interface.
- Query all of the above states

The API should follow REST conventions, and the endpoints should be documented in the API documentation.

## Backend

The emulated controller shall be connected to the ALVR backend as a virtual controller, so that the backend can treat it as a real controller. The backend will receive the emulated controller input and forward it to the VR application. Web API and user inputs are merged.