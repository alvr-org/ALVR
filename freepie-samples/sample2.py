global offset

if starting:
  offset = [0.0, 0.0, 0.0]

speed = 0.01
offset[0] += mouse.deltaX * speed
offset[2] += mouse.deltaY * speed

# You need to set this variable, to enable position
alvr.override_head_position = True

alvr.head_position[0] = alvr.input_head_position[0] + offset[0]
alvr.head_position[1] = alvr.input_head_position[1] + offset[1]
alvr.head_position[2] = alvr.input_head_position[2] + offset[2]

# You need to set this variable, to enable position
alvr.override_controller_position = True

alvr.controller_position[0] = alvr.input_controller_position[0] + offset[0]
alvr.controller_position[1] = alvr.input_controller_position[1] + offset[1]
alvr.controller_position[2] = alvr.input_controller_position[2] + offset[2]

