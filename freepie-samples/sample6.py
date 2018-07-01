# This sample does not work. Only a conceptual code.

if starting:
  alvr.two_controllers = True

# controll second controller by smartphone rotation
alvr.controller_orientation[1][0] = android[0].yaw
alvr.controller_orientation[1][1] = android[0].pitch
alvr.controller_orientation[1][2] = android[0].roll
