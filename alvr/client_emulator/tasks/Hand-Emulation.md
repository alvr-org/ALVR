# Hand Emulation

Based on the [Controller Emulation](Controller-Emulation.md) task, this task is designed to emulate hand tracking using the ALVR client. It allows users to simulate hand movements and gestures in VR applications that support hand tracking.

The rough approach for hand input is similar:

- We need a hands section on the toolbar, with L and R toggles for the left and right hands and a Reset button to reset the hand pose, as well as display toggle
- The hands will have a same on screen representation as the controllers (colored circle and letters) and the same exact 6DoF movement and rotation toolbar. This behavior will be exact same, including the off screen handling. This will define the base coordinate system for the hands (or more precisely the palm), but will not affect the individual poses of the fingers.
- To distinguish between hands and controllers, the hands will display LH and RH indicators, while the controllers will display LC and RC indicators in the overlay.
- The controller and hand inputs are mutually exclusive, so if turn on the left hand, the left controller will be disabled and vice versa. The same applies for the right hand and right controller. Untoggling the hand / controller however will NOT reenable the other.
- The hands will have a different panel displayed on the bottom left and right corners of the screen. There the user will be able to select from a number of hand gestures.
- The hand gestures will be defined in a JSON file, each gesture will have a name, a description (these will appear as tooltips), an icon (svg, referenced from JSON, shown on the button for the pose) and a set of finger / joint poses
- The user can select a single pose by clicking on the button generated from the JSON file, the poses will be organized into a grid. Selecting a pose will smoothly transition the hand from the current pose to the selected pose. The transition will be animated over a - JSON - configurable time (default 0.5s). The user can also select a "Idle" pose, which will be the default pose for the hands.
- The hands must be displayed as skeletal models in the 3D view, the hand model can itself be simple or cartoon like, however the joints must be rigged and skinned. I suggest to download a free hand model from the internet which is already rigged and skinned, and use that as a base for the hand model. The model will be provided in a GLTF file and attached to the hand profile in the settings (see below). The model will be loaded at runtime, so the user can add new models without recompiling the code.

The following hand poses will be provided by default:
- Idle: all fingers relaxed, open palm
- Grasp: all fingers curled into a fist
- Point: index finger extended, all other fingers curled into a fist
- Pinch: thumb and index finger touching, all other fingers curled into a fist

The joint positions should be defined in a simple way, that is the joint positions are defined in a local coordinate system, where the origin is at the base of the palm, the articulations are limited to the anatomical possibilities of the human hand. A possible solution:
- For each finger we have a curled vs. relaxed state 0 is fully relaxed and 1 is fully curled, in between the joints realistically curl up gradually.
- The thumb is a special case as it can also move sideways (e.g. in relaxed state it can parallel to the other fingers 0 - or it can be perpendicular to the other fingers 1). This articulation also applies in curled state.
- The fingers can also be spread apart, this is a single value for the whole hand, 0 is fully together and 1 is fully spread apart. This articulation also applies in curled state.
- If you can find better, more anatomically correct ways to define the joint positions, please propose them. The goal is to have a simple and intuitive way to define the joint positions, that can be easily understood by non-experts and does not require too many values and can reproduce all poses in a simple way (we could technically represent this with 3DoF rotation of each joint, but that is very complex to work with, and allows many impossible poses).
