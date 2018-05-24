xcopy /y ALVR\bin\Release\ALVR.exe release-files
xcopy /y ALVR\bin\Release\*.dll release-files
mkdir release-files\driver
mkdir release-files\driver\bin
mkdir release-files\driver\bin\win64
mkdir release-files\driver\resources
copy /y driver.vrdrivermanifest release-files\driver
copy /y driver_uninstall.bat release-files\driver
copy /y driver_install.bat release-files\driver
xcopy /y/s/e/i resources release-files\driver\resources
xcopy /y/s/e/i bin\win64\* release-files\driver\bin\win64
