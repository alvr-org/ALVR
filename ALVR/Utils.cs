using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Threading.Tasks;

namespace ALVR
{
    class Utils
    {
        public static string GetDriverPath()
        {
            string exePath = Assembly.GetEntryAssembly().Location;
            string driverPath = Path.GetDirectoryName(exePath) + "\\driver";
            if (Environment.GetCommandLineArgs().Length >= 2)
            {
                driverPath = Environment.GetCommandLineArgs()[1];
            }
            return driverPath;
        }

        public static float ParseFloat(string s)
        {
            float f = 0.0f;
            float.TryParse(s, out f);
            return f;
        }

        // Execute command without showing command prompt window.
        public static Process ExecuteProcess(string path, string args)
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
