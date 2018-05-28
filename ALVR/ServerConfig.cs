using Codeplex.Data;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    class ServerConfig
    {
        public static readonly int DEFAULT_BITRATE = 30;
        public static readonly int DEFAULT_WIDTH = 2048;
        public static readonly int[] supportedWidth = new int[] { 1024, 1536, 2048 };

        public int bitrate { get; private set; } // in Mbps
        public int renderWidth { get; private set; }

        public ServerConfig()
        {
        }

        public void Load()
        {
            string config = Utils.GetConfigPath();

            FileStream stream = null;
            try
            {
                stream = new FileStream(config, FileMode.Open, FileAccess.Read);
            }
            catch (Exception e)
            {
                MessageBox.Show("Error opning " + config + "\r\nPlease check existence of driver folder.");
                Environment.Exit(-1);
                return;
            }
            dynamic configJson = DynamicJson.Parse(stream);
            string nvencOptions = configJson.driver_alvr_server.nvencOptions;
            var m = Regex.Match(nvencOptions, ".*-bitrate ([^ ]+)M.*");
            if (!m.Success)
            {
                bitrate = DEFAULT_BITRATE;
            }
            else
            {
                try
                {
                    bitrate = int.Parse(m.Groups[1].Value);
                }
                catch (Exception e)
                {
                    bitrate = DEFAULT_BITRATE;
                }
            }

            renderWidth = (int)configJson.driver_alvr_server.renderWidth;
            if (!supportedWidth.Contains(renderWidth))
            {
                renderWidth = DEFAULT_WIDTH;
            }
        }

        public void Save(int abitrate, int awidth)
        {
            bitrate = abitrate;
            renderWidth = awidth;

            string config = Utils.GetConfigPath();
            dynamic configJson;
            try
            {
                using (FileStream stream = new FileStream(config, FileMode.Open, FileAccess.Read))
                {
                    configJson = DynamicJson.Parse(stream);
                }
                configJson.driver_alvr_server.nvencOptions = "-codec h264 -preset ll_hq -rc cbr_ll_hq -gop 120 -fps 60 -bitrate " + abitrate + "M -maxbitrate " + abitrate + "M";

                configJson.driver_alvr_server.renderWidth = awidth;
                configJson.driver_alvr_server.renderHeight = awidth / 2;

                configJson.driver_alvr_server.debugOutputDir = Utils.GetDriverPath();

                using (FileStream stream = new FileStream(config, FileMode.Create, FileAccess.Write))
                {
                    var bytes = Encoding.UTF8.GetBytes(configJson.ToString());
                    stream.Write(bytes, 0, bytes.Length);
                }
            }
            catch (Exception e)
            {
                MessageBox.Show("Error opning " + config + "\r\nPlease check existence of driver folder.");
                Environment.Exit(-1);
                return;
            }
        }
    }
}
