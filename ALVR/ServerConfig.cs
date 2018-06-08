using Codeplex.Data;
using Microsoft.CSharp.RuntimeBinder;
using System;
using System.Collections.Generic;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    class ServerConfig
    {
        private static readonly string APP_FILEMAPPING_NAME = "ALVR_DRIVER_FILEMAPPING_0B124897-7730-4B84-AA32-088E9B92851F";

        public class Resolution
        {
            public int width { get; set; }
            public string display { get { return width + " x " + (width / 2); } }
            public override string ToString()
            {
                return display;
            }
        }
        public static readonly Resolution[] supportedResolutions = {
            new Resolution { width = 1024 }
            , new Resolution { width = 1536 }
            , new Resolution { width = 2048 }
            , new Resolution { width = 2560 }
            , new Resolution { width = 2880 }
            , new Resolution { width = 3072 }
        };

        public class ComboBoxCustomItem
        {
            public ComboBoxCustomItem(string s, int val)
            {
                text = s;
                value = val;
            }
            private readonly string text;
            public int value { get; private set; }

            public override string ToString()
            {
                return text;
            }
        }

        // From OpenVR EVRButtonId
        public static readonly ComboBoxCustomItem[] supportedButtons = {
            new ComboBoxCustomItem("None", -1)
            ,new ComboBoxCustomItem("System", 0)
            ,new ComboBoxCustomItem("ApplicationMenu", 1)
            ,new ComboBoxCustomItem("Grip", 2)
            ,new ComboBoxCustomItem("DPad_Left", 3)
            ,new ComboBoxCustomItem("DPad_Up", 4)
            ,new ComboBoxCustomItem("DPad_Right", 5)
            ,new ComboBoxCustomItem("DPad_Down", 6)
            ,new ComboBoxCustomItem("A Button", 7)
            ,new ComboBoxCustomItem("B Button", 8)
            ,new ComboBoxCustomItem("X Button", 9)
            ,new ComboBoxCustomItem("Y Button", 10)
            ,new ComboBoxCustomItem("Trackpad", 28) // 28
            ,new ComboBoxCustomItem("Trigger", 24) // 24
            ,new ComboBoxCustomItem("Shoulder Left", 13)
            ,new ComboBoxCustomItem("Shoulder Right", 14)
            ,new ComboBoxCustomItem("Joystick Left", 15)
            ,new ComboBoxCustomItem("Joystick Right", 18)
            ,new ComboBoxCustomItem("Back", 21)
            ,new ComboBoxCustomItem("Guide", 22)
            ,new ComboBoxCustomItem("Start", 23)
        };
        public static readonly string[] supportedRecenterButton = new string[] {
            "None", "Trigger", "Trackpad click", "Trackpad touch"//, "Back short-press"
        };
        
        public ServerConfig()
        {
        }

        public static int FindButton(int button)
        {
            for (var i = 0; i < supportedButtons.Length; i++)
            {
                if (supportedButtons[i].value == button)
                {
                    return i;
                }
            }
            return -1;
        }

        public int GetBufferSizeKB()
        {
            if (Properties.Settings.Default.bufferSize == 5)
            {
                return 200;
            }
            // Map 0 - 100 to 100kB - 2000kB
            return Properties.Settings.Default.bufferSize * 1900 / 100 + 100;
        }

        public bool Save()
        {
            try
            {
                dynamic configJson = new DynamicJson();
                var driver = configJson.driver_alvr_server;
                driver.serialNumber = "ALVR-001";
                driver.modelNumber = "ALVR driver server";
                driver.adapterIndex = 0;
                driver.IPD = 0.064;
                driver.secondsFromVsyncToPhotons = 0.005;
                driver.displayFrequency = 60;
                driver.listenPort = 9944;
                driver.listenHost = "0.0.0.0";
                driver.sendingTimeslotUs = 500;
                driver.limitTimeslotPackets = 0;
                driver.controlListenPort = 9944;
                driver.controlListenHost = "127.0.0.1";
                driver.useKeyedMutex = true;
                driver.controllerModelNumber = "Gear VR Controller";
                driver.controllerSerialNumber = "Controller-001";

                driver.nvencOptions = "-codec h264 -preset ll_hq -rc cbr_ll_hq -gop 120 -fps 60 -bitrate "
                    + Properties.Settings.Default.bitrate + "M -maxbitrate " + Properties.Settings.Default.bitrate + "M";

                driver.renderWidth = Properties.Settings.Default.renderWidth;
                driver.renderHeight = Properties.Settings.Default.renderWidth / 2;

                driver.debugOutputDir = Utils.GetDriverPath();
                driver.debugLog = Properties.Settings.Default.debugLog;

                driver.clientRecvBufferSize = GetBufferSizeKB() * 1000;
                driver.enableController = Properties.Settings.Default.enableController;
                driver.controllerTriggerMode = Properties.Settings.Default.controllerTriggerMode;
                driver.controllerTrackpadClickMode = Properties.Settings.Default.controllerTrackpadClickMode;
                driver.controllerTrackpadTouchMode = Properties.Settings.Default.controllerTrackpadTouchMode;

                // 0=Disabled, 1=Trigger, 2=Trackpad Click, 3=Trackpad Touch
                driver.controllerRecenterButton = Properties.Settings.Default.controllerRecenterButton;
                driver.useTrackingReference = Properties.Settings.Default.useTrackingReference;

                byte[] bytes = Encoding.UTF8.GetBytes(configJson.ToString());
                using (var mapped = MemoryMappedFile.CreateOrOpen(APP_FILEMAPPING_NAME, sizeof(int) + bytes.Length))
                {
                    using (var mappedStream = mapped.CreateViewStream())
                    {
                        mappedStream.Write(BitConverter.GetBytes(bytes.Length), 0, sizeof(int));
                        mappedStream.Write(bytes, 0, bytes.Length);
                    }
                }
            }
            catch (Exception e)
            {
                MessageBox.Show("Error on creating filemapping.\r\nPlease check the status of vrserver.exe and retry.");
                return false;
            }
            return true;
        }
    }
}
