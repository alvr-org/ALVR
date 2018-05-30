using Codeplex.Data;
using Microsoft.CSharp.RuntimeBinder;
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
        public static readonly int DEFAULT_BUFFER_SIZE = 200 * 1000; // 200kB
        public static readonly int DEFAULT_TRIGGER_MODE = 33;
        public static readonly int DEFAULT_TRACKPAD_CLICK_MODE = 7;
        public static readonly int DEFAULT_TRACKPAD_TOUCH_MODE = 7;
        public static readonly int DEFAULT_RECENTER_BUTTON = 0; // 0=Disabled, 1=Trigger, 2=Trackpad Click
        public static readonly int[] supportedWidth = new int[] { 1024, 1536, 2048 };
        // From OpenVR EVRButtonId
        public static readonly string[] supportedButtons = new string[] {
            "System"
            , "ApplicationMenu"
            , "Grip"
            , "DPad_Left"
            , "DPad_Up"
            , "DPad_Right"
            , "DPad_Down"
            , "A Button"
            , "Touchpad"
            , "Trigger"
        };
        public static readonly int[] supportedButtonId = new int[] { 0, 1, 2, 3, 4, 5, 6, 7, 32, 33 };
        public static readonly string[] supportedRecenterButton = new string[] {
            "None", "Trigger", "Trackpad click"
        };

        public int bitrate { get; set; } // in Mbps
        public int renderWidth { get; set; } // in pixels
        public int bufferSize { get; set; } // in bytes

        public int controllerTriggerMode { get; set; }
        public int controllerTrackpadClickMode { get; set; }
        public int controllerTrackpadTouchMode { get; set; }
        public int controllerRecenterButton { get; set; }

        public ServerConfig()
        {
        }

        public bool Load()
        {
            string config = Utils.GetConfigPath();

            FileStream stream = null;
            try
            {
                stream = new FileStream(config, FileMode.Open, FileAccess.Read);
            }
            catch (Exception e)
            {
                MessageBox.Show("Error on opening " + config + "\r\nPlease check existence of driver folder.");
                return false;
            }
            dynamic configJson = DynamicJson.Parse(stream);
            bitrate = DEFAULT_BITRATE;

            try
            {
                string nvencOptions = configJson.driver_alvr_server.nvencOptions;
                var m = Regex.Match(nvencOptions, ".*-bitrate ([^ ]+)M.*");
                if (m.Success)
                {
                    bitrate = int.Parse(m.Groups[1].Value);
                }
            }
            catch (Exception e)
            {
            }

            renderWidth = DEFAULT_WIDTH;
            try
            {
                renderWidth = (int)configJson.driver_alvr_server.renderWidth;
                if (!supportedWidth.Contains(renderWidth))
                {
                    renderWidth = DEFAULT_WIDTH;
                }
            }
            catch (Exception e)
            {
            }

            try
            {
                bufferSize = (int)configJson.driver_alvr_server.clientRecvBufferSize;
            }
            catch (RuntimeBinderException e)
            {
                bufferSize = DEFAULT_BUFFER_SIZE;
            }

            try
            {
                controllerTriggerMode = (int)configJson.driver_alvr_server.controllerTriggerMode;
            }
            catch (RuntimeBinderException e)
            {
                controllerTriggerMode = DEFAULT_TRIGGER_MODE;
            }

            try
            {
                controllerTrackpadClickMode = (int)configJson.driver_alvr_server.controllerTrackpadClickMode;
            }
            catch (RuntimeBinderException e)
            {
                controllerTrackpadClickMode = DEFAULT_TRACKPAD_CLICK_MODE;
            }

            try
            {
                controllerTrackpadTouchMode = (int)configJson.driver_alvr_server.controllerTrackpadTouchMode;
            }
            catch (RuntimeBinderException e)
            {
                controllerTrackpadTouchMode = DEFAULT_TRACKPAD_TOUCH_MODE;
            }

            try
            {
                controllerRecenterButton = (int)configJson.driver_alvr_server.controllerRecenterButton;
            }
            catch (RuntimeBinderException e)
            {
                controllerRecenterButton = DEFAULT_RECENTER_BUTTON;
            }
            return true;
        }

        public bool Save(bool adebugLog)
        {
            string config = Utils.GetConfigPath();
            dynamic configJson;
            try
            {
                using (FileStream stream = new FileStream(config, FileMode.Open, FileAccess.Read))
                {
                    configJson = DynamicJson.Parse(stream);
                }
                configJson.driver_alvr_server.nvencOptions = "-codec h264 -preset ll_hq -rc cbr_ll_hq -gop 120 -fps 60 -bitrate " + bitrate + "M -maxbitrate " + bitrate + "M";

                configJson.driver_alvr_server.renderWidth = renderWidth;
                configJson.driver_alvr_server.renderHeight = renderWidth / 2;

                configJson.driver_alvr_server.debugOutputDir = Utils.GetDriverPath();
                configJson.driver_alvr_server.debugLog = adebugLog;

                configJson.driver_alvr_server.clientRecvBufferSize = bufferSize;
                configJson.driver_alvr_server.controllerTriggerMode = controllerTriggerMode;
                configJson.driver_alvr_server.controllerTrackpadClickMode = controllerTrackpadClickMode;
                configJson.driver_alvr_server.controllerTrackpadTouchMode = controllerTrackpadTouchMode;
                configJson.driver_alvr_server.controllerRecenterButton = controllerRecenterButton;

                using (FileStream stream = new FileStream(config, FileMode.Create, FileAccess.Write))
                {
                    var bytes = Encoding.UTF8.GetBytes(configJson.ToString());
                    stream.Write(bytes, 0, bytes.Length);
                }
            }
            catch (Exception e)
            {
                MessageBox.Show("Error on opening " + config + "\r\nPlease check existence of driver folder.");
                return false;
            }
            return true;
        }
    }
}
