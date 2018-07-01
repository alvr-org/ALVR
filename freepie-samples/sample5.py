
if starting:
  # enable second controller
  alvr.two_controllers = True

# select controller by shift key
controller = 1 if keyboard.getKeyDown(Key.LeftShift) else 0

# Click trackpad of first controller by "C" key
alvr.buttons[controller][alvr.Id("trackpad_click")] = keyboard.getKeyDown(Key.C)
alvr.buttons[controller][alvr.Id("trackpad_touch")] = keyboard.getKeyDown(Key.C)

# Move trackpad position by arrow keys
if keyboard.getKeyDown(Key.LeftArrow):
  alvr.trackpad[controller][0] = -1.0
  alvr.trackpad[controller][1] = 0.0
elif keyboard.getKeyDown(Key.UpArrow):
  alvr.trackpad[controller][0] = 0.0
  alvr.trackpad[controller][1] = 1.0
elif keyboard.getKeyDown(Key.RightArrow):
  alvr.trackpad[controller][0] = 1.0
  alvr.trackpad[controller][1] = 0.0
elif keyboard.getKeyDown(Key.DownArrow):
  alvr.trackpad[controller][0] = 0.0
  alvr.trackpad[controller][1] = -1.0
