Controller tracking will always be difficult. There are many factors that can influence the latency and motion prediction. It's not something like a constant `100 ms`, but can greatly depend on your movements and even the movement of the headset.

There are many parameters that influence the movement that can be changed:

- Tracking is currently async to the rendering and running at `3 * 72 = 216 Hz`
- Movement prediction is set to `0` to get the latest tracking info -> no prediction on the quest
- Tracking info is sent to SteamVR
- Tracking info is fed into SteamVR with an offset of `10 ms` to enable SteamVR pose prediction
- The tracking point on the Quest is different than the point on the Rift S. Angular acceleration and linear acceleration of the controller needed to be transformed to the new reference.

There is a trade off between "fast but wobbly", "overshooting controllers" and "controllers that have a certain latency". Depending on system to system, the current settings are perfectly playable for games like Skyrim, Fallout, or Arizona Sunshine. Although for games like Beat Saber, stock settings might be an issue.

You can change the `10 ms` offset for SteamVR in the "Other" tab of ALVR (Controller Pose Offset). The parameter defines how old the data that is fed into SteamVR is and controls the SteamVR pose prediction. Setting it to `0` disables all predictions.

The default is `0.01`, equaling to `10 ms`; It's the amount of time needed to being able to swing a sword in Skyrim without feeling awkward. It's possible that this value depends on the game or user. With it being exposed in the control panel, you're able change it during runtime.
