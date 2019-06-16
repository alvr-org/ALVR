mkdir release-files

rd /s/q release-files\ALVR

mkdir release-files\ALVR
mkdir release-files\ALVR\logs
xcopy /y ALVR\bin\Release\ALVR.exe release-files\ALVR
xcopy /y CrashReport\bin\Release\CrashReport.exe release-files\ALVR
xcopy /y ALVR\bin\Release\*.dll release-files\ALVR
xcopy /y/s/e/i driver release-files\ALVR\driver

mkdir release-files\ALVR\driver\bin\win64
xcopy /y x64\Release\driver_alvr_server.dll release-files\ALVR\driver\bin\win64

xcopy /y libswresample\lib\*.dll release-files\ALVR\driver\bin\win64
xcopy /y/s/e/i freepie-samples release-files\ALVR\freepie-samples
xcopy /y ALVRFreePIE\bin\Release\ALVRFreePIE.dll release-files\ALVR
xcopy /y add_firewall_rules.bat release-files\ALVR
xcopy /y remove_firewall_rules.bat release-files\ALVR

rd /s/q release-files\ALVR-dev
mkdir release-files\ALVR-dev
xcopy /y CUDA\x64\Release\CUDA.pdb release-files\ALVR-dev
xcopy /y x64\Release\CUDA.lib release-files\ALVR-dev
xcopy /y x64\Release\driver_alvr_server.pdb release-files\ALVR-dev

cd release-files
7z a -tzip ALVR-portable-v.zip ALVR
7z a -tzip ALVR-pdb-lib-v.zip ALVR-dev
cd ..

"C:\Program Files (x86)\Inno Setup 5\ISCC.exe" installer\installer.iss

pause
