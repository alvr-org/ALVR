While most games do work without any problems, some do only work partially or not at all. This includes

- headset not found
- warped image
- controller not tracking
- buttons not working
- ...

Most of the time its the overly specific initialization of the game towards a specific headset that breaks the game.
For example, Vivecraft broke because ALVR reported the headset manufacturer as "Oculus driver 1.38.0" and not as "Oculus".
In general, this is a rather bad practice as all relevant data can be accessed trough SteamVR and the game should not make assumptions based on the manufacturer of the hmd. There are many different fields that a game could require to run.

Nonetheless, we want to play and support those games. 
Problem is, that we don't own all games. This is a Open Source without any funding. We can not buy any games just to fix a bug. In the case of Vivecraft, one user (thanks @Avencore) was generous to gift us a copy and the bug could be fixed.
There are no guaranties! Neither on the time it will take nor if the bug will ever be fixed! Please contact us before buying anything.
