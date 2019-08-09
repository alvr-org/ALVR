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

        public static readonly int DEFAULT_SCALE_INDEX = 3; // 100%
        public static readonly int[] supportedScales = { 25, 50, 75, 100, 125, 150, 175, 200 };

        public static readonly int DEFAULT_REFRESHRATE = 72;
        public static readonly int DEFAULT_WIDTH = 2432;
        public static readonly int DEFAULT_HEIGHT = 1344;

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
            ,new ComboBoxCustomItem("DPad_Left", 5)
            ,new ComboBoxCustomItem("DPad_Up", 6)
            ,new ComboBoxCustomItem("DPad_Right", 7)
            ,new ComboBoxCustomItem("DPad_Down", 8)
            ,new ComboBoxCustomItem("A Button", 9)
            ,new ComboBoxCustomItem("B Button", 11)
            ,new ComboBoxCustomItem("X Button", 13)
            ,new ComboBoxCustomItem("Y Button", 15)
            ,new ComboBoxCustomItem("Trackpad", 39)
            ,new ComboBoxCustomItem("Trigger", 34)
            ,new ComboBoxCustomItem("Shoulder Left", 19)
            ,new ComboBoxCustomItem("Shoulder Right", 20)
            ,new ComboBoxCustomItem("Joystick Left", 21)
            ,new ComboBoxCustomItem("Joystick Right", 24)
            ,new ComboBoxCustomItem("Back", 31)
            ,new ComboBoxCustomItem("Guide", 32)
            ,new ComboBoxCustomItem("Start", 33)
        };
        public static readonly string[] supportedRecenterButton = new string[] {
            "None", "Trigger", "Trackpad click", "Trackpad touch", "Back"
        };
        public static readonly int[] recenterButtonIndex = new int[] {
            -1, 34, 39, 40, 31
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
            return 0;
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

        public bool Save(DeviceDescriptor device)
        {
            try
            {
                var c = Properties.Settings.Default;
                dynamic driverConfig = new DynamicJson();
                if (device != null && device.HasTouchController)
                {
                    driverConfig.serialNumber = "WMHD000X000XXX";
                    driverConfig.trackingSystemName = "oculus";
                    driverConfig.modelNumber = "Oculus Rift S";
                    driverConfig.manufacturerName = "Oculus driver 1.38.0";
                    driverConfig.renderModelName = "generic_hmd";
                    driverConfig.registeredDeviceType = "oculus_rifts";
                }
                else
                {
                    driverConfig.serialNumber = "HTCVive-001";
                    driverConfig.trackingSystemName = "Vive Tracker";
                    driverConfig.modelNumber = "ALVR driver server";
                    driverConfig.manufacturerName = "HTC";
                    driverConfig.renderModelName = "generic_hmd";
                    driverConfig.registeredDeviceType = "vive";
                }
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

                driverConfig.codec = c.codec; // 0: H264, 1: H265
                driverConfig.encodeBitrateInMBits = c.bitrate;

                if (device == null)
                {
                    driverConfig.refreshRate = DEFAULT_REFRESHRATE;
                    driverConfig.renderWidth = DEFAULT_WIDTH;
                    driverConfig.renderHeight = DEFAULT_HEIGHT;

                    driverConfig.autoConnectHost = "";
                    driverConfig.autoConnectPort = 0;

                    driverConfig.eyeFov = new double[] { 45, 45, 45, 45, 45, 45, 45, 45 };
                }
                else
                {
                    driverConfig.refreshRate = device.RefreshRates[0] == 0 ? DEFAULT_REFRESHRATE : device.RefreshRates[0];
                    driverConfig.renderWidth = device.DefaultWidth * c.resolutionScale / 100;
                    driverConfig.renderHeight = device.DefaultHeight * c.resolutionScale / 100;

                    driverConfig.autoConnectHost = device.ClientHost;
                    driverConfig.autoConnectPort = device.ClientPort;

                    driverConfig.eyeFov = device.EyeFov;
                }
                driverConfig.disableThrottling = c.disableThrottling;

                driverConfig.enableSound = c.enableSound && c.soundDevice != "";
                driverConfig.soundDevice = c.soundDevice;

                driverConfig.debugOutputDir = Utils.GetOutputPath();
                driverConfig.debugLog = c.debugLog;
                driverConfig.debugFrameIndex = false;
                driverConfig.debugFrameOutput = false;
                driverConfig.debugCaptureOutput = c.debugCaptureOutput;
                driverConfig.useKeyedMutex = true;

                driverConfig.clientRecvBufferSize = GetBufferSizeKB() * 1000;
                driverConfig.frameQueueSize = GetFrameQueueSize(c.suppressFrameDrop);

                driverConfig.force60HZ = c.force60Hz;

                driverConfig.enableController = c.enableController;
                if(device != null && device.HasTouchController)
                {
                    driverConfig.controllerTrackingSystemName = "oculus";
                    driverConfig.controllerManufacturerName = "Oculus";
                    driverConfig.controllerModelNumber = "Oculus Rift S";
                    driverConfig.controllerRenderModelNameLeft = "oculus_rifts_controller_left";
                    driverConfig.controllerRenderModelNameRight = "oculus_rifts_controller_right";
                    driverConfig.controllerSerialNumber = "WMHD000X000XXX_Controller";
                    driverConfig.controllerType = "oculus_touch";
                    driverConfig.controllerRegisteredDeviceType = "oculus_touch";
                    driverConfig.controllerLegacyInputProfile = "oculus_touch";
                    driverConfig.controllerInputProfilePath = "{alvr_server}/input/touch_profile.json";
                }
                else
                {
                    driverConfig.controllerTrackingSystemName = "ALVR Remote Controller";
                    driverConfig.controllerManufacturerName = "ALVR";
                    driverConfig.controllerModelNumber = "ALVR Remote Controller";
                    driverConfig.controllerRenderModelNameLeft = "vr_controller_vive_1_5";
                    driverConfig.controllerRenderModelNameRight = "vr_controller_vive_1_5";
                    driverConfig.controllerSerialNumber = "ALVR Remote Controller";
                    driverConfig.controllerType = "vive_controller";
                    driverConfig.controllerRegisteredDeviceType = "vive_controller";
                    driverConfig.controllerLegacyInputProfile = "vive_controller";
                    driverConfig.controllerInputProfilePath = "{alvr_server}/input/vive_controller_profile.json";
                }
                driverConfig.controllerTriggerMode = c.controllerTriggerMode;
                driverConfig.controllerTrackpadClickMode = c.controllerTrackpadClickMode;
                driverConfig.controllerTrackpadTouchMode = c.controllerTrackpadTouchMode;
                driverConfig.controllerBackMode = c.controllerBackMode;

                // -1=Disabled, other=ALVR Input id
                driverConfig.controllerRecenterButton = recenterButtonIndex[c.controllerRecenterButton];
                driverConfig.useTrackingReference = c.useTrackingReference;

                driverConfig.enableOffsetPos = c.useOffsetPos;
                driverConfig.offsetPosX = Utils.ParseFloat(c.offsetPosX);
                driverConfig.offsetPosY = Utils.ParseFloat(c.offsetPosY);
                driverConfig.offsetPosZ = Utils.ParseFloat(c.offsetPosZ);

                driverConfig.trackingFrameOffset = Utils.ParseInt(c.trackingFrameOffset);
                driverConfig.controllerPoseOffset = Utils.ParseFloat(c.controllerPoseOffset);

                byte[] bytes = Encoding.UTF8.GetBytes(driverConfig.ToString());
                memoryMappedFile = MemoryMappedFile.CreateOrOpen(APP_FILEMAPPING_NAME, sizeof(int) + bytes.Length);

                using (var mappedStream = memoryMappedFile.CreateViewStream())
                {
                    mappedStream.Write(BitConverter.GetBytes(bytes.Length), 0, sizeof(int));
                    mappedStream.Write(bytes, 0, bytes.Length);
                }

            }
            catch (Exception)
            {
                MessageBox.Show("Error on creating filemapping.\r\nPlease check the status of vrserver.exe and retry.");
                return false;
            }
            return true;
        }
    }
}
