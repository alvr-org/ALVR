using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    class DriverInstaller
    {
        // Execute "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" adddriver "%~dp0
        public static bool InstallDriver()
        {
            RegistryKey regkey = Registry.ClassesRoot.OpenSubKey(@"vrmonitor\Shell\Open\Command", false);
            if (regkey == null)
            {
                MessageBox.Show("SteamVR is not installed.\r\n(Registry HKEY_CLASSES_ROOT\\vrmonitor\\Shell\\Open\\Command was not found.)\r\nPlease install and retry.");
                return false;
            }
            string path = (string)regkey.GetValue("");

            var m = Regex.Match(path, "^\"(.+)bin\\\\([^\\\\]+)\\\\vrmonitor.exe\" \"%1\"$");
            if (!m.Success)
            {
                MessageBox.Show("Invalid value in registry HKEY_CLASSES_ROOT\\vrmonitor\\Shell\\Open\\Command.");
                return false;
            }
            string vrpathreg = m.Groups[1].Value + @"bin\win32\vrpathreg.exe";

            string driverPath = Utils.GetDriverPath();
            if (!Directory.Exists(driverPath))
            {
                MessageBox.Show("Driver path: " + driverPath + "\r\nis not found! Please check install location.");
                return false;
            }
            // This is for compatibility to driver_uninstall.bat
            driverPath += "\\\\";

            ExecuteProcess(vrpathreg, "adddriver \"" + driverPath + "\"");

            return true;
        }

        // Execute vrpathreg without showing command prompt window.
        private static void ExecuteProcess(string path, string args)
        {
            ProcessStartInfo startInfo = new ProcessStartInfo();
            startInfo.FileName = path;
            startInfo.Arguments = args;
            startInfo.RedirectStandardOutput = true;
            startInfo.RedirectStandardError = true;
            startInfo.UseShellExecute = false;
            startInfo.CreateNoWindow = true;
            startInfo.WindowStyle = ProcessWindowStyle.Hidden;

            Process processTemp = new Process();
            processTemp.StartInfo = startInfo;
            processTemp.EnableRaisingEvents = true;
            try
            {
                processTemp.Start();
            }
            catch (Exception e)
            {
                throw;
            }
        }
    }
}
