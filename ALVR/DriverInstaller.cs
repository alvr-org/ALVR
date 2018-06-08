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
            string vrpathreg = GetVRPathRegPath();
            if (vrpathreg == null)
            {
                return false;
            }

            string driverPath = Utils.GetDriverPath();
            if (!Directory.Exists(driverPath))
            {
                MessageBox.Show("Driver path: " + driverPath + "\r\nis not found! Please check install location.");
                return false;
            }
            // This is for compatibility to driver_uninstall.bat
            driverPath += "\\\\";

            ExecuteProcess(vrpathreg, "adddriver \"" + driverPath + "\"").WaitForExit();

            return true;
        }

        public static bool UninstallDriver()
        {
            string vrpathreg = GetVRPathRegPath();
            if (vrpathreg == null)
            {
                return false;
            }

            string driverPath = Utils.GetDriverPath();
            // We don't check existence when uninstalling.
            // This is for compatibility to driver_uninstall.bat
            driverPath += "\\\\";

            ExecuteProcess(vrpathreg, "removedriver \"" + driverPath + "\"").WaitForExit();

            return true;
        }

        public static bool CheckInstalled()
        {
            string vrpathreg = GetVRPathRegPath();
            if (vrpathreg == null)
            {
                throw new Exception();
            }

            string driverPath = Utils.GetDriverPath();
            driverPath += "\\";

            var process = ExecuteProcess(vrpathreg, "show");
            while (!process.StandardOutput.EndOfStream)
            {
                string line = process.StandardOutput.ReadLine();
                if (line.Trim("\n\t ".ToCharArray()) == driverPath)
                {
                    return true;
                }
            }
            return false;
        }

        public static bool ListDrivers()
        {
            string vrpathreg = GetVRPathRegPath();
            if (vrpathreg == null)
            {
                throw new Exception();
            }

            string driverPath = Utils.GetDriverPath();
            driverPath += "\\";

            var process = ExecuteProcess(vrpathreg, "show");
            string list = process.StandardOutput.ReadToEnd();
            int index = list.IndexOf("External Drivers:\r\n");
            if (index != -1)
            {
                list = "Installed driver list:\r\n" + list.Substring(index + "External Drivers:\r\n".Length);
            }
            MessageBox.Show(list, "ALVR");
            return true;
        }

        private static string GetVRPathRegPath()
        {
            RegistryKey regkey = Registry.ClassesRoot.OpenSubKey(@"vrmonitor\Shell\Open\Command", false);
            if (regkey == null)
            {
                MessageBox.Show("SteamVR is not installed.\r\n(Registry HKEY_CLASSES_ROOT\\vrmonitor\\Shell\\Open\\Command was not found.)\r\nPlease install and retry.");
                return null;
            }
            string path = (string)regkey.GetValue("");

            var m = Regex.Match(path, "^\"(.+)bin\\\\([^\\\\]+)\\\\vrmonitor.exe\" \"%1\"$");
            if (!m.Success)
            {
                MessageBox.Show("Invalid value in registry HKEY_CLASSES_ROOT\\vrmonitor\\Shell\\Open\\Command.");
                return null;
            }
            return m.Groups[1].Value + @"bin\win32\vrpathreg.exe";
        }

        // Execute vrpathreg without showing command prompt window.
        private static Process ExecuteProcess(string path, string args)
        {
            ProcessStartInfo startInfo = new ProcessStartInfo();
            startInfo.FileName = path;
            startInfo.Arguments = args;
            startInfo.RedirectStandardOutput = true;
            startInfo.RedirectStandardError = true;
            startInfo.UseShellExecute = false;
            startInfo.CreateNoWindow = true;
            startInfo.WindowStyle = ProcessWindowStyle.Hidden;

            Process process = new Process();
            process.StartInfo = startInfo;
            process.EnableRaisingEvents = true;
            try
            {
                process.Start();
            }
            catch (Exception e)
            {
                throw;
            }
            return process;
        }
    }
}
