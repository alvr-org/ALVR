mkdir release-files
mkdir release-files\ALVR
xcopy /y ALVR\bin\Release\ALVR.exe release-files\ALVR
xcopy /y CrashReport\bin\Release\CrashReport.exe release-files\ALVR
xcopy /y ALVR\bin\Release\*.dll release-files\ALVR
xcopy /y/s/e/i driver release-files\ALVR\driver
