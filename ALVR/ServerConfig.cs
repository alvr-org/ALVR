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
        public static readonly bool DEFAULT_ENABLE_CONTROLLER = true;
        public static readonly int DEFAULT_TRIGGER_MODE = 24;
        public static readonly int DEFAULT_TRACKPAD_CLICK_MODE = 28;
        public static readonly int DEFAULT_TRACKPAD_TOUCH_MODE = 29;
        public static readonly int DEFAULT_RECENTER_BUTTON = 0; // 0=Disabled, 1=Trigger, 2=Trackpad Click, 3=Trackpad Touch
        public static readonly int[] supportedWidth = new int[] { 1024, 1536, 2048 };
        // From OpenVR EVRButtonId
        public static readonly string[] supportedButtons = new string[] {
            "None"
            ,"System"
            , "ApplicationMenu"
            , "Grip"
            , "DPad_Left"
            , "DPad_Up"
            , "DPad_Right"
            , "DPad_Down"
            , "A Button"
            , "B Button"
            , "X Button"
            , "Y Button" // 10
            , "Trackpad" // 28
            , "Trigger" // 24
            , "Shoulder Left"
            , "Shoulder Right"
            , "Joystick Left"
            , "Joystick Right"
            , "Back"
            , "Guide"
            , "Start"
        };
        public static readonly int[] supportedButtonId = new int[] { -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 /* Y */
            , 28, 24, 13, 14, 15, 18, 21, 22, 23 };
        public static readonly string[] supportedRecenterButton = new string[] {
            "None", "Trigger", "Trackpad click", "Trackpad touch"//, "Back short-press"
        };

        public int bitrate { get; set; } // in Mbps
        public int renderWidth { get; set; } // in pixels
        public int bufferSize { get; set; } // in bytes

        public bool enableController { get; set; }
        public int controllerTriggerMode { get; set; }
        public int controllerTrackpadClickMode { get; set; }
        public int controllerTrackpadTouchMode { get; set; }
        public int controllerRecenterButton { get; set; }

        public ServerConfig()
        {
        }

        public void EnsureSupportedValue(int []list, int value)
        {
            if (Array.IndexOf(list, value) == -1)
            {
                throw new NotSupportedException();
            }
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

            try
            {
                renderWidth = (int)configJson.driver_alvr_server.renderWidth;
                EnsureSupportedValue(supportedWidth, renderWidth);
            }
            catch (Exception e)
            {
                renderWidth = DEFAULT_WIDTH;
            }

            try
            {
                bufferSize = (int)configJson.driver_alvr_server.clientRecvBufferSize;
            }
            catch (RuntimeBinderException e)
            {
                bufferSize = DEFAULT_BUFFER_SIZE;
            }

            //
            // Controller settings
            //

            try
            {
                enableController = (bool)configJson.driver_alvr_server.enableController;
            }
            catch (Exception e)
            {
                enableController = DEFAULT_ENABLE_CONTROLLER;
            }


            try
            {
                controllerTriggerMode = (int)configJson.driver_alvr_server.controllerTriggerMode;
                EnsureSupportedValue(supportedButtonId, controllerTriggerMode);
            }
            catch (Exception e)
            {
                controllerTriggerMode = DEFAULT_TRIGGER_MODE;
            }

            try
            {
                controllerTrackpadClickMode = (int)configJson.driver_alvr_server.controllerTrackpadClickMode;
                EnsureSupportedValue(supportedButtonId, controllerTrackpadClickMode);
            }
            catch (Exception e)
            {
                controllerTrackpadClickMode = DEFAULT_TRACKPAD_CLICK_MODE;
            }

            try
            {
                //controllerTrackpadTouchMode = (int)configJson.driver_alvr_server.controllerTrackpadTouchMode;
                // We only support "Trackpad touch" value on controllerTrackpadTouchMode
                controllerTrackpadTouchMode = DEFAULT_TRACKPAD_TOUCH_MODE;
                //EnsureSupportedValue(supportedButtonId, controllerTrackpadTouchMode);
            }
            catch (Exception e)
            {
                controllerTrackpadTouchMode = DEFAULT_TRACKPAD_TOUCH_MODE;
            }

            try
            {
                controllerRecenterButton = (int)configJson.driver_alvr_server.controllerRecenterButton;
                if (controllerRecenterButton < 0 || 3 < controllerRecenterButton)
                {
                    controllerRecenterButton = DEFAULT_RECENTER_BUTTON;
                }
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
                configJson.driver_alvr_server.enableController = enableController;
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
