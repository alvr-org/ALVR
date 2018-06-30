using FreePIE.Core.Contracts;
using System;
using System.Collections.Generic;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Text;
using System.Threading;

namespace ALVRFreePIE
{
    [GlobalType(Type = typeof(ALVRFreePIEPluginGlobal))]
    public class ALVRFreePIEPlugin : IPlugin
    {
        ALVRFreePIEPluginGlobal global;
        public object CreateGlobal()
        {
            return global = new ALVRFreePIEPluginGlobal(this);
        }

        public Action Start()
        {
            return null;
        }

        public void Stop()
        {
        }

        public event EventHandler Started;

        public string FriendlyName
        {
            get { return "ALVR FreePIE Plugin"; }
        }

        public bool GetProperty(int index, IPluginProperty property)
        {
            if (index == 0)
            {
                property.Name = "SomeProperty";
                property.Caption = "ALVR FreePIE plugin is installed";
                property.HelpText = "ALVR FreePIE plugin is installed";

                property.Choices.Add("ALVR FreePIE plugin is installed", 1);

                property.DefaultValue = 1;
                return true;
            }
            return false;
        }

        public bool SetProperties(Dictionary<string, object> properties)
        {
            return true;
        }

        public void DoBeforeNextExecute()
        {
            CheckMemoryMappedFileExistence();
            if (memoryMappedFile == null)
            {
                return;
            }
            try
            {
                mutex.WaitOne(-1);

                UInt32 inputControllerButtons = 0;

                using (var mappedStream = memoryMappedFile.CreateViewStream())
                {
                    var reader = new BinaryReader(mappedStream);

                    UInt32 version = reader.ReadUInt32();
                    if (version == ALVR_FREEPIE_SIGNATURE_V2)
                    {
                        reader.ReadUInt32();
                        // Head orientation
                        global.input_head_orientation[0] = reader.ReadDouble();
                        global.input_head_orientation[1] = reader.ReadDouble();
                        global.input_head_orientation[2] = reader.ReadDouble();
                        // Controller orientation
                        global.input_controller_orientation[0] = reader.ReadDouble();
                        global.input_controller_orientation[1] = reader.ReadDouble();
                        global.input_controller_orientation[2] = reader.ReadDouble();
                        // Head position
                        global.input_head_position[0] = reader.ReadDouble();
                        global.input_head_position[1] = reader.ReadDouble();
                        global.input_head_position[2] = reader.ReadDouble();
                        // Controller position
                        global.input_controller_position[0] = reader.ReadDouble();
                        global.input_controller_position[1] = reader.ReadDouble();
                        global.input_controller_position[2] = reader.ReadDouble();

                        global.input_trackpad[0] = reader.ReadDouble();
                        global.input_trackpad[1] = reader.ReadDouble();

                        inputControllerButtons = reader.ReadUInt16();
                    }
                }

                using (var mappedStream = memoryMappedFile.CreateViewStream())
                {
                    mappedStream.Seek(sizeof(UInt32), SeekOrigin.Current);

                    UInt32 flags = 0;
                    if (global.override_head_orientation)
                    {
                        flags |= ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION;
                    }
                    if (global.override_controller_orientation)
                    {
                        flags |= ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION0;
                    }
                    if (global.override_head_position)
                    {
                        flags |= ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION;
                    }
                    if (global.override_controller_position)
                    {
                        flags |= ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION0;
                    }
                    flags |= ALVR_FREEPIE_FLAG_OVERRIDE_BUTTONS;

                    mappedStream.Write(BitConverter.GetBytes(flags), 0, sizeof(UInt32));

                    mappedStream.Seek(sizeof(double) * 14 + sizeof(UInt16), SeekOrigin.Current);

                    mappedStream.Write(BitConverter.GetBytes(global.two_controllers ? 2 : 1), 0, sizeof(UInt16));

                    UInt32[] buttons = { 0, 0 };
                    for (int j = 0; j < 2; j++)
                    {
                        for (int i = 0; i < BUTTONS.Length; i++)
                        {
                            buttons[j] |= global.buttons[j][i] ? (1U << i) : 0U;
                        }
                    }
                    mappedStream.Write(BitConverter.GetBytes(buttons[0]), 0, sizeof(UInt32));
                    mappedStream.Write(BitConverter.GetBytes(buttons[1]), 0, sizeof(UInt32));

                    mappedStream.Write(BitConverter.GetBytes(global.head_orientation[0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.head_orientation[1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.head_orientation[2]), 0, sizeof(double));

                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[0][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[0][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[0][2]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[1][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[1][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[1][2]), 0, sizeof(double));

                    mappedStream.Write(BitConverter.GetBytes(global.head_position[0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.head_position[1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.head_position[2]), 0, sizeof(double));

                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[0][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[0][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[0][2]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[1][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[1][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.controller_position[1][2]), 0, sizeof(double));

                    mappedStream.Write(BitConverter.GetBytes(global.trigger[0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trigger[1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trigger_left[0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trigger_left[1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trigger_right[0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trigger_right[1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_left[0][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_left[1][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_left[0][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_left[1][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_right[0][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_right[1][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_right[0][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.joystick_right[1][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trackpad[0][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trackpad[1][0]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trackpad[0][1]), 0, sizeof(double));
                    mappedStream.Write(BitConverter.GetBytes(global.trackpad[1][1]), 0, sizeof(double));

                    var bytes = Encoding.UTF8.GetBytes(global.message);
                    mappedStream.Write(bytes, 0, Math.Min(bytes.Length, ALVR_FREEPIE_MESSAGE_LENGTH - 1));
                    mappedStream.WriteByte(0);
                }

                for (int i = 0; i < INPUT_BUTTONS.Length; i++)
                {
                    global.input_buttons[i] = (inputControllerButtons & (1U << i)) != 0;
                }
            }
            finally
            {
                mutex.ReleaseMutex();
            }
        }

        private void CheckMemoryMappedFileExistence()
        {
            if (memoryMappedFile != null)
            {
                return;
            }
            try
            {
                memoryMappedFile = MemoryMappedFile.OpenExisting(ALVR_FREEPIE_FILEMAPPING_NAME);
                mutex = new Mutex(false, ALVR_FREEPIE_MUTEX_NAME);
            }
            catch (Exception e)
            {
            }
        }

        static readonly string ALVR_FREEPIE_FILEMAPPING_NAME = "ALVR_FREEPIE_FILEMAPPING_13B65572-591A-4248-A2F6-BAC2D89EE3B8";
        static readonly string ALVR_FREEPIE_MUTEX_NAME = "ALVR_FREEPIE_MUTEX_AA77F1C3-86E4-4EF9-AAA2-5C40CF380D7A";

        static readonly UInt32 ALVR_FREEPIE_SIGNATURE_V2 = 0x11223345;

        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION = 1 << 0;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION0 = 1 << 1;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION = 1 << 2;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION0 = 1 << 3;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_BUTTONS = 1 << 4;

        static readonly int ALVR_FREEPIE_MESSAGE_LENGTH = 512;

        public static readonly string[] INPUT_BUTTONS = {"trackpad_click", "trackpad_touch", "trigger", "back", "volume_up", "volume_down"};
        public static readonly string[] BUTTONS = {"system", "application_menu", "grip"
                , "dpad_left", "dpad_up", "dpad_right", "dpad_down"
                , "a", "b", "x", "y"
                , "trackpad_click", "trackpad_touch", "trigger", "shoulder_left", "shoulder_right"
        , "joystick_left", "joystick_right", "back", "guide", "start"};

        MemoryMappedFile memoryMappedFile;
        Mutex mutex;
    }

    [Global(Name = "alvr")]
    public class ALVRFreePIEPluginGlobal
    {
        private readonly ALVRFreePIEPlugin plugin;

        public ALVRFreePIEPluginGlobal(ALVRFreePIEPlugin plugin)
        {
            this.plugin = plugin;
        }

        public int InputId(string key)
        {
            for (int i = 0; i < ALVRFreePIEPlugin.INPUT_BUTTONS.Length; i++)
            {
                if (ALVRFreePIEPlugin.INPUT_BUTTONS[i] == key)
                {
                    return i;
                }
            }
            return -1;
        }

        public int Id(string key)
        {
            for (int i = 0; i < ALVRFreePIEPlugin.BUTTONS.Length; i++)
            {
                if (ALVRFreePIEPlugin.BUTTONS[i] == key)
                {
                    return i;
                }
            }
            return -1;
        }

        public bool two_controllers { get; set; }

        public bool override_head_orientation { get; set; }
        public bool override_controller_orientation { get; set; }
        public bool override_head_position { get; set; }
        public bool override_controller_position { get; set; }

        // yaw pitch roll
        public double[] input_head_orientation { get; set; } = new double[3];
        public double[] input_controller_orientation { get; set; } = new double[3];
        // x y z
        public double[] input_head_position { get; set; } = new double[3];
        public double[] input_controller_position { get; set; } = new double[3];

        // yaw pitch roll
        public double[] head_orientation { get; set; } = new double[3];
        public double[][] controller_orientation { get; set; } = { new double[3], new double[3] };
        // x y z
        public double[] head_position { get; set; } = new double[3];
        public double[][] controller_position { get; set; } = { new double[3], new double[3] };


        public bool[] input_buttons { get; set; } = new bool[6];
        public bool[][] buttons { get; set; } = { new bool[ALVRFreePIEPlugin.BUTTONS.Length], new bool[ALVRFreePIEPlugin.BUTTONS.Length] };

        // x y
        public double[] input_trackpad { get; set; } = new double[2];

        public double[] trigger { get; set; } = new double[2];
        public double[] trigger_left { get; set; } = new double[2];
        public double[] trigger_right { get; set; } = new double[2];
        // joystick_left[controller][xy]
        public double[][] joystick_left { get; set; } = { new double[2], new double[2] };
        // x y
        public double[][] joystick_right { get; set; } = { new double[2], new double[2] };
        // x y
        public double[][] trackpad { get; set; } = { new double[2], new double[2] };

        public string message { get; set; } = "";

    }
}
