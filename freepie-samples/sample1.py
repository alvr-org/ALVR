import math, time

global prev_back, mode, offset, message_time

if starting:
  prev_back = False
  mode = 0
  offset = [0.0, 0.0, 0.0]
  message_time = 0.0


map = [["system", Key.G], ["application_menu", Key.X], ["trigger", Key.T], ["a", Key.V], ["b", Key.B], ["x", Key.N], ["y", Key.M]
, ["grip", Key.F1], ["trackpad_click", Key.F2], ["back", Key.F3], ["guide", Key.F4], ["start", Key.F5]
, ["dpad_left", Key.F6], ["dpad_up", Key.F7], ["dpad_right", Key.F8], ["dpad_down", Key.F9], ["trackpad_touch", Key.F10]]

for k in map:
  alvr.buttons[alvr.Id(k[0])] = keyboard.getKeyDown(k[1])

if prev_back != alvr.input_buttons[alvr.InputId("back")]:
  prev_back = alvr.input_buttons[alvr.InputId("back")]
  if alvr.input_buttons[alvr.InputId("back")]:
    mode = (mode + 1) % 3
    alvr.message = "mode " + str(mode)
    message_time = time.time()

if time.time() - message_time > 2:
  alvr.message = ""

if mode == 0:
  # trackpad guesture
  alvr.buttons[alvr.Id("system")] = alvr.buttons[alvr.Id("system")] or alvr.input_buttons[alvr.InputId("trigger")]
  alvr.buttons[alvr.Id("application_menu")] = alvr.buttons[alvr.Id("application_menu")] or alvr.input_buttons[alvr.InputId("back")]
  if alvr.input_buttons[alvr.InputId("trackpad_click")]:
    if alvr.input_trackpad[0] + alvr.input_trackpad[1] > 0.0:
      if alvr.input_trackpad[0] - alvr.input_trackpad[1] > 0.0:
        alvr.buttons[alvr.Id("trigger")] = True
        alvr.trigger = 1.0
      else:
        alvr.controller_position[0] += 0.3
    else:
      if alvr.input_trackpad[0] - alvr.input_trackpad[1] > 0.0:
        alvr.controller_position[0] += -0.3
      else:
        alvr.buttons[alvr.Id("application_menu")] = True
elif mode == 1:
  # fly (buggy)
  if alvr.input_buttons[alvr.InputId("trackpad_click")]:
    theta = alvr.input_controller_orientation[1]
    speed = 0.01
    offset[0] += speed * (-math.cos(theta) * alvr.input_trackpad[0] - math.sin(theta) * alvr.input_trackpad[1])
    offset[2] += speed * (-math.sin(theta) * alvr.input_trackpad[0] + math.cos(theta) * alvr.input_trackpad[1])

  alvr.buttons[alvr.Id("trigger")] = alvr.buttons[alvr.Id("trigger")] or alvr.input_buttons[alvr.InputId("trigger")]
elif mode == 2:
  # passthrough
  alvr.buttons[alvr.Id("trackpad_click")] = alvr.buttons[alvr.Id("trackpad_click")] or alvr.input_buttons[alvr.InputId("trackpad_click")]
  alvr.buttons[alvr.Id("trackpad_touch")] = alvr.buttons[alvr.Id("trackpad_touch")] or alvr.input_buttons[alvr.InputId("trackpad_touch")]
  alvr.buttons[alvr.Id("trigger")] = alvr.buttons[alvr.Id("trigger")] or alvr.input_buttons[alvr.InputId("trigger")]
  alvr.trackpad[0] = alvr.input_trackpad[0]
  alvr.trackpad[1] = alvr.input_trackpad[1]

alvr.trigger = 1.0 if alvr.buttons[alvr.Id("trigger")] else 0.0


alvr.override_head_position = True

alvr.head_position[0] = alvr.input_head_position[0] + offset[0]
alvr.head_position[1] = alvr.input_head_position[1] + offset[1]
alvr.head_position[2] = alvr.input_head_position[2] + offset[2]

alvr.override_controller_position = True

alvr.controller_position[0] = alvr.input_controller_position[0] + offset[0]
alvr.controller_position[1] = alvr.input_controller_position[1] + offset[1]
alvr.controller_position[2] = alvr.input_controller_position[2] + offset[2]

if False:
  diagnostics.watch(alvr.input_head_orientation[0])
  diagnostics.watch(alvr.input_head_orientation[1])
  diagnostics.watch(alvr.input_head_orientation[2])
  
  diagnostics.watch(alvr.input_controller_orientation[0])
  diagnostics.watch(alvr.input_controller_orientation[1])
  diagnostics.watch(alvr.input_controller_orientation[2])
  
  diagnostics.watch(alvr.input_head_position[0])
  diagnostics.watch(alvr.input_head_position[1])
  diagnostics.watch(alvr.input_head_position[2])
  
  diagnostics.watch(alvr.input_controller_position[0])
  diagnostics.watch(alvr.input_controller_position[1])
  diagnostics.watch(alvr.input_controller_position[2])
  
  diagnostics.watch(alvr.input_trackpad[0])
  diagnostics.watch(alvr.input_trackpad[1])
  
  diagnostics.watch(alvr.input_buttons[0])
  diagnostics.watch(alvr.input_buttons[1])
  diagnostics.watch(alvr.input_buttons[2])
  diagnostics.watch(alvr.input_buttons[3])
  diagnostics.watch(alvr.input_buttons[4])
  diagnostics.watch(alvr.input_buttons[5])

