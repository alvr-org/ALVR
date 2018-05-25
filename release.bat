mkdir release-files
mkdir release-files\ALVR
xcopy /y ALVR\bin\Release\ALVR.exe release-files\ALVR
xcopy /y ALVR\bin\Release\*.dll release-files\ALVR
mkdir release-files\ALVR\driver
mkdir release-files\ALVR\driver\bin
mkdir release-files\ALVR\driver\bin\win64
mkdir release-files\ALVR\driver\resources
copy /y driver.vrdrivermanifest release-files\ALVR\driver
copy /y driver_uninstall.bat release-files\ALVR\driver
copy /y driver_install.bat release-files\ALVR\driver
xcopy /y/s/e/i resources release-files\ALVR\driver\resources
xcopy /y/s/e/i bin\win64\* release-files\ALVR\driver\bin\win64
