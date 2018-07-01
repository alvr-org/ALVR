import math, time

global prev_back, mode, offset, message_time

def sign(x): return 1 if x >= 0 else -1

# conjugate quaternion
def conj(q):
  return [-q[0], -q[1], -q[2], q[3]]

# multiplication of quaternion
def multiply(a, b):
  x0, y0, z0, w0 = a
  x1, y1, z1, w1 = b
  return [x1 * w0 - y1 * z0 + z1 * y0 + w1 * x0,
      x1 * z0 + y1 * w0 - z1 * x0 + w1 * y0,
      -x1 * y0 + y1 * x0 + z1 * w0 + w1 * z0,
      -x1 * x0 - y1 * y0 - z1 * z0 + w1 * w0]

# convert quaternion to euler
def quaternion2euler(q):
  yaw_pitch_roll = [0.0, 0.0, 0.0]
  # roll (x-axis rotation)
  sinr = +2.0 * (q[3] * q[0] + q[1] * q[2])
  cosr = +1.0 - 2.0 * (q[0] * q[0] + q[1] * q[1])
  yaw_pitch_roll[2] = atan2(sinr, cosr)

  # pitch (y-axis rotation)
  sinp = +2.0 * (q[3] * q[1] - q[2] * q[0])
  if (fabs(sinp) >= 1):
    yaw_pitch_roll[1] = math.copysign(M_PI / 2, sinp)
  else:
    yaw_pitch_roll[1] = math.asin(sinp)

  # yaw (z-axis rotation)
  siny = +2.0 * (q[3] * q[2] + q[0] * q[1]);
  cosy = +1.0 - 2.0 * (q[1] * q[1] + q[2] * q[2]);
  yaw_pitch_roll[0] = math.atan2(siny, cosy);

  return yaw_pitch_roll

# convert euler to quaternion
def euler2quaternion(yaw_pitch_roll):
  cy = math.cos(yaw_pitch_roll[0] * 0.5);
  sy = math.sin(yaw_pitch_roll[0] * 0.5);
  cr = math.cos(yaw_pitch_roll[2] * 0.5);
  sr = math.sin(yaw_pitch_roll[2] * 0.5);
  cp = math.cos(yaw_pitch_roll[1] * 0.5);
  sp = math.sin(yaw_pitch_roll[1] * 0.5);

  return [cy * sr * cp - sy * cr * sp,
  cy * cr * sp + sy * sr * cp,
  sy * cr * cp - cy * sr * sp,
  cy * cr * cp + sy * sr * sp]

# rotate specified vector using yaw_pitch_roll
def rotatevec(yaw_pitch_roll, vec):
  q = euler2quaternion(yaw_pitch_roll)
  return multiply(multiply(q, vec), conj(q))

if starting:
  prev_back = False
  mode = 0
  offset = [0.0, 0.0, 0.0]
  message_time = 0.0
  alvr.two_controllers = True
  controller = 0

# change target controller
if keyboard.getPressed(Key.Z):
  controller = 1 - controller

map = [["system", Key.G], ["application_menu", Key.X], ["trigger", Key.T], ["a", Key.V], ["b", Key.B], ["x", Key.N], ["y", Key.M]
, ["grip", Key.F1], ["trackpad_click", Key.F2], ["back", Key.F3], ["guide", Key.F4], ["start", Key.F5]
, ["dpad_left", Key.F6], ["dpad_up", Key.F7], ["dpad_right", Key.F8], ["dpad_down", Key.F9], ["trackpad_touch", Key.F10]]

for k in map:
  alvr.buttons[controller][alvr.Id(k[0])] = keyboard.getKeyDown(k[1])

if prev_back != alvr.input_buttons[alvr.InputId("back")]:
  prev_back = alvr.input_buttons[alvr.InputId("back")]
  if alvr.input_buttons[alvr.InputId("back")]:
    mode = (mode + 1) % 3
    # show messageo on display
    alvr.message = "mode " + str(mode)
    message_time = time.time()

if time.time() - message_time > 2:
  # remove message after 2 seconds
  alvr.message = ""

if mode == 0:
  # trackpad guesture mode
  alvr.buttons[controller][alvr.Id("trigger")] = alvr.buttons[controller][alvr.Id("trigger")] or alvr.input_buttons[alvr.InputId("trigger")]
  #alvr.buttons[controller][alvr.Id("application_menu")] = alvr.buttons[controller][alvr.Id("application_menu")] or alvr.input_buttons[alvr.InputId("back")]

  if alvr.input_buttons[alvr.InputId("trackpad_click")]:
    if alvr.input_trackpad[0] + alvr.input_trackpad[1] > 0.0:
      if alvr.input_trackpad[0] - alvr.input_trackpad[1] > 0.0:
        # right
        alvr.buttons[controller][alvr.Id("system")] = True
      else:
        # top
        alvr.buttons[controller][alvr.Id("trackpad_click")] = True
        alvr.buttons[controller][alvr.Id("trackpad_touch")] = True
    else:
      if alvr.input_trackpad[0] - alvr.input_trackpad[1] > 0.0:
        # bottom
        alvr.buttons[controller][alvr.Id("grip")] = True
      else:
        # left
        alvr.buttons[controller][alvr.Id("application_menu")] = True
elif mode == 1:
  # fly mode (buggy)
  # press upper half of trackpad to forward. bottom half to back
  if alvr.input_buttons[alvr.InputId("trackpad_click")]:
    outvec = rotatevec(alvr.input_controller_orientation, [0, 0, -1, 0])
    speed = 0.002 * sign(alvr.input_trackpad[1])
    offset[0] += speed * outvec[0]
    offset[1] += speed * outvec[1]
    offset[2] += speed * outvec[2]
  if alvr.input_buttons[alvr.InputId("trigger")] and alvr.input_buttons[alvr.InputId("trackpad_click")]:
    offset = [0.0, 0.0, 0.0]

  alvr.buttons[controller][alvr.Id("trigger")] = alvr.buttons[controller][alvr.Id("trigger")] or alvr.input_buttons[alvr.InputId("trigger")]
elif mode == 2:
  # passthrough mode
  alvr.buttons[controller][alvr.Id("trackpad_click")] = alvr.buttons[controller][alvr.Id("trackpad_click")] or alvr.input_buttons[alvr.InputId("trackpad_click")]
  alvr.buttons[controller][alvr.Id("trackpad_touch")] = alvr.buttons[controller][alvr.Id("trackpad_touch")] or alvr.input_buttons[alvr.InputId("trackpad_touch")]
  alvr.buttons[controller][alvr.Id("trigger")] = alvr.buttons[controller][alvr.Id("trigger")] or alvr.input_buttons[alvr.InputId("trigger")]
  alvr.trackpad[controller][0] = alvr.input_trackpad[0]
  alvr.trackpad[controller][1] = alvr.input_trackpad[1]

# You need to set trigger value correctly to get trigger click work
alvr.trigger[controller] = 1.0 if alvr.buttons[controller][alvr.Id("trigger")] else 0.0

alvr.override_head_position = True

alvr.head_position[0] = alvr.input_head_position[0] + offset[0]
alvr.head_position[1] = alvr.input_head_position[1] + offset[1]
alvr.head_position[2] = alvr.input_head_position[2] + offset[2]

alvr.override_controller_position = True

alvr.controller_position[controller][0] = alvr.input_controller_position[0] + offset[0]
alvr.controller_position[controller][1] = alvr.input_controller_position[1] + offset[1]
alvr.controller_position[controller][2] = alvr.input_controller_position[2] + offset[2]
#alvr.controller_position[1-controller][0] = alvr.input_controller_position[0] + offset[0] + 0.1
#alvr.controller_position[1-controller][1] = alvr.input_controller_position[1] + offset[1] + 0.1
#alvr.controller_position[1-controller][2] = alvr.input_controller_position[2] + offset[2] + 0.1
#alvr.controller_orientation[1-controller][0] = android[0].yaw
#alvr.controller_orientation[1-controller][1] = android[0].pitch
#alvr.controller_orientation[1-controller][2] = android[0].roll

alvr.override_controller_orientation = True
alvr.controller_orientation[controller][0] = alvr.input_controller_orientation[0]
alvr.controller_orientation[controller][1] = alvr.input_controller_orientation[1]
alvr.controller_orientation[controller][2] = alvr.input_controller_orientation[2]

if True:
  # watch variables on FreePIE debugger
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

  diagnostics.watch(alvr.head_position[0])
  diagnostics.watch(alvr.head_position[1])
  diagnostics.watch(alvr.head_position[2])
