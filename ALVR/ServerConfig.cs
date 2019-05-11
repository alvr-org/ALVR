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

        public class EyeFov
        {
            public readonly double[] eyeFov;
            public EyeFov(double[] eyeFov)
            {
                this.eyeFov = eyeFov;
            }
        }
        public static readonly EyeFov EYE_FOV_GEARVR = new EyeFov(new double[] { 45, 45, 45, 45, 45, 45, 45, 45 });
        public static readonly EyeFov EYE_FOV_DAYDREAMVIEW = new EyeFov(new double[] { 53, 45, 53, 44, 45, 53, 53, 44 });
        public static readonly EyeFov EYE_FOV_MIRAGESOLO = new EyeFov(new double[] { 46, 45, 46, 46, 45, 46, 46, 46 });
        public class Resolution
        {
            public int width { get; set; }
            public int height { get; set; }
            public string display;
            public EyeFov eyeFov;
            public override string ToString()
            {
                return display;
            }
            public Resolution(int width, int height, EyeFov eyeFov)
            {
                this.width = width;
                this.height = height;
                display = width + " x " + height;
                this.eyeFov = eyeFov;
            }
            public Resolution(int width, int height, string label, EyeFov eyeFov)
            {
                this.width = width;
                this.height = height;
                display = width + " x " + height + " " + label + "";
                this.eyeFov = eyeFov;
            }
        }
        public static readonly Resolution[] supportedResolutions = {
            new Resolution(1024, 512, EYE_FOV_GEARVR)
            , new Resolution(1536, 768, EYE_FOV_GEARVR)
            , new Resolution(2048, 1024, EYE_FOV_GEARVR)
            , new Resolution(2560, 1280, EYE_FOV_GEARVR)
            , new Resolution(2880, 1440, EYE_FOV_GEARVR)
            , new Resolution(3072, 1536, EYE_FOV_GEARVR)
            , new Resolution(2432, 1344, "Quest", EYE_FOV_GEARVR)
            , new Resolution(2260, 1150, "Mirage Solo(Mid)", EYE_FOV_MIRAGESOLO)
            , new Resolution(3390, 1726, "Mirage Solo(Max)", EYE_FOV_MIRAGESOLO)
            , new Resolution(2565, 1256, "DaydreamView(Mid)", EYE_FOV_DAYDREAMVIEW)
            , new Resolution(3848, 1884, "DaydreamView(Max)", EYE_FOV_DAYDREAMVIEW)
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
            "None", "Trigger", "Trackpad click", "Trackpad touch", "Back"
        };

        public static readonly ComboBoxCustomItem[] supportedCodecs = {
            new ComboBoxCustomItem("H.264 AVC", 0),
            new ComboBoxCustomItem("H.265 HEVC", 1)
        };

        MemoryMappedFile memoryMappedFile;

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

        public int GetFrameQueueSize(bool suppressFrameDrop)
        {
            return suppressFrameDrop ? 5 : 1;
        }

        public bool Save()
        {
            try
            {
                dynamic driverConfig = new DynamicJson();
                driverConfig.serialNumber = "ALVR-001";
                driverConfig.modelNumber = "ALVR driver server";
                driverConfig.adapterIndex = 0;
                driverConfig.IPD = 0.063;
                driverConfig.secondsFromVsyncToPhotons = 0.005;
                driverConfig.listenPort = 9944;
                driverConfig.listenHost = "0.0.0.0";
                driverConfig.sendingTimeslotUs = 500;
                driverConfig.limitTimeslotPackets = 0;
                driverConfig.controlListenPort = 9944;
                driverConfig.controlListenHost = "127.0.0.1";
                driverConfig.useKeyedMutex = true;
                driverConfig.controllerTrackingSystemName = "ALVR Remote Controller";
                driverConfig.controllerManufacturerName = "ALVR";
                driverConfig.controllerModelNumber = "ALVR Remote Controller";
                driverConfig.controllerRenderModelName = "vr_controller_vive_1_5";
                driverConfig.controllerSerialNumber = "ALVR Remote Controller";

                driverConfig.codec = Properties.Settings.Default.codec; // 0: H264, 1: H265
                driverConfig.encodeBitrateInMBits = Properties.Settings.Default.bitrate;

                driverConfig.refreshRate = 60;
                driverConfig.renderWidth = Properties.Settings.Default.renderWidth;
                driverConfig.renderHeight = Properties.Settings.Default.renderHeight;

                driverConfig.eyeFov = Properties.Settings.Default.eyeFov;

                driverConfig.enableSound = Properties.Settings.Default.enableSound && Properties.Settings.Default.soundDevice != "";
                driverConfig.soundDevice = Properties.Settings.Default.soundDevice;

                driverConfig.debugOutputDir = Utils.GetOutputPath();
                driverConfig.debugLog = Properties.Settings.Default.debugLog;
                driverConfig.debugFrameIndex = false;
                driverConfig.debugFrameOutput = false;
                driverConfig.debugCaptureOutput = Properties.Settings.Default.debugCaptureOutput;
                driverConfig.useKeyedMutex = true;

                driverConfig.clientRecvBufferSize = GetBufferSizeKB() * 1000;
                driverConfig.frameQueueSize = GetFrameQueueSize(Properties.Settings.Default.suppressFrameDrop);

                driverConfig.force60HZ = Properties.Settings.Default.force60Hz;

                driverConfig.enableController = Properties.Settings.Default.enableController;
                driverConfig.controllerTriggerMode = Properties.Settings.Default.controllerTriggerMode;
                driverConfig.controllerTrackpadClickMode = Properties.Settings.Default.controllerTrackpadClickMode;
                driverConfig.controllerTrackpadTouchMode = Properties.Settings.Default.controllerTrackpadTouchMode;
                driverConfig.controllerBackMode = Properties.Settings.Default.controllerBackMode;

                // 0=Disabled, 1=Trigger, 2=Trackpad Click, 3=Trackpad Touch, 4=Back
                driverConfig.controllerRecenterButton = Properties.Settings.Default.controllerRecenterButton;
                driverConfig.useTrackingReference = Properties.Settings.Default.useTrackingReference;

                driverConfig.enableOffsetPos = Properties.Settings.Default.useOffsetPos;
                driverConfig.offsetPosX = Utils.ParseFloat(Properties.Settings.Default.offsetPosX);
                driverConfig.offsetPosY = Utils.ParseFloat(Properties.Settings.Default.offsetPosY);
                driverConfig.offsetPosZ = Utils.ParseFloat(Properties.Settings.Default.offsetPosZ);

                driverConfig.trackingFrameOffset = Utils.ParseInt(Properties.Settings.Default.trackingFrameOffset);

                byte[] bytes = Encoding.UTF8.GetBytes(driverConfig.ToString());
                memoryMappedFile = MemoryMappedFile.CreateOrOpen(APP_FILEMAPPING_NAME, sizeof(int) + bytes.Length);

                using (var mappedStream = memoryMappedFile.CreateViewStream())
                {
                    mappedStream.Write(BitConverter.GetBytes(bytes.Length), 0, sizeof(int));
                    mappedStream.Write(bytes, 0, bytes.Length);
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
