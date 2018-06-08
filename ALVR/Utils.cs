using System;
using System.Collections.Generic;
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
    }
}
