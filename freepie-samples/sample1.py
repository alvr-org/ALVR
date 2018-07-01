# Click trackpad of first controller by "C" key
alvr.buttons[0][alvr.Id("trackpad_click")] = keyboard.getKeyDown(Key.C)
alvr.buttons[0][alvr.Id("trackpad_touch")] = keyboard.getKeyDown(Key.C)

# Move trackpad position by arrow keys
if keyboard.getKeyDown(Key.LeftArrow):
  alvr.trackpad[0][0] = -1.0
  alvr.trackpad[0][1] = 0.0
elif keyboard.getKeyDown(Key.UpArrow):
  alvr.trackpad[0][0] = 0.0
  alvr.trackpad[0][1] = 1.0
elif keyboard.getKeyDown(Key.RightArrow):
  alvr.trackpad[0][0] = 1.0
  alvr.trackpad[0][1] = 0.0
elif keyboard.getKeyDown(Key.DownArrow):
  alvr.trackpad[0][0] = 0.0
  alvr.trackpad[0][1] = -1.0
