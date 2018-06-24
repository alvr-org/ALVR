using FreePIE.Core.Contracts;
using System;
using System.Collections.Generic;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Threading;

namespace ALVRFreePIE
{
    [GlobalType(Type = typeof(ALVRFreePIEPlugin))]
    public class ALVRFreePIEPlugin
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
            return false;
        }

        public bool SetProperties(Dictionary<string, object> properties)
        {
            return true;
        }

        public void DoBeforeNextExecute()
        {
            CheckMemoryMappedFileExistence();
            if (memoryMappedFile != null)
            {
                try
                {
                    mutex.WaitOne(0);

                    UInt32 flags = 0;
                    UInt32 controllerOverrideButtons = 0;
                    UInt32 controllerButtons = 0;

                    using (var mappedStream = memoryMappedFile.CreateViewStream())
                    {
                        var reader = new BinaryReader(mappedStream);

                        UInt32 version = reader.ReadUInt32();
                        if (version == ALVR_FREEPIE_SIGNATURE_V1)
                        {
                            flags = reader.ReadUInt32();
                            // Head orientation
                            double yaw = reader.ReadDouble();
                            double pitch = reader.ReadDouble();
                            double roll = reader.ReadDouble();
                            // Head position
                            double[] head_position = new double[3];
                            head_position[0] = reader.ReadDouble();
                            head_position[1] = reader.ReadDouble();
                            head_position[2] = reader.ReadDouble();

                            controllerOverrideButtons = reader.ReadUInt32();
                            controllerButtons = reader.ReadUInt32();
                        }
                    }

                    using (var mappedStream = memoryMappedFile.CreateViewStream())
                    {
                        mappedStream.Seek(sizeof(UInt32), SeekOrigin.Current);
                        if (global.override_head_orientation) {
                            flags |= ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION;
                        }
                        if (global.override_head_position)
                        {
                            flags |= ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION;
                        }
                        if (global.override_controller_orientation)
                        {
                            flags |= ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION;
                        }
                        if (global.override_controller_position)
                        {
                            flags |= ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION;
                        }

                        mappedStream.Write(BitConverter.GetBytes(flags), 0, sizeof(UInt32));

                        if (global.override_head_orientation)
                        {
                            mappedStream.Write(BitConverter.GetBytes(global.head_orientation[0]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.head_orientation[1]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.head_orientation[2]), 0, sizeof(double));
                        }
                        else
                        {
                            mappedStream.Seek(sizeof(double) * 3, SeekOrigin.Current);
                        }
                        if (global.override_head_position)
                        {
                            mappedStream.Write(BitConverter.GetBytes(global.head_position[0]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.head_position[1]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.head_position[2]), 0, sizeof(double));
                        }
                        else
                        {
                            mappedStream.Seek(sizeof(double) * 3, SeekOrigin.Current);
                        }
                        if (global.override_controller_orientation)
                        {
                            mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[0]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[1]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.controller_orientation[2]), 0, sizeof(double));
                        }
                        else
                        {
                            mappedStream.Seek(sizeof(double) * 3, SeekOrigin.Current);
                        }
                        if (global.override_controller_position)
                        {
                            mappedStream.Write(BitConverter.GetBytes(global.controller_position[0]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.controller_position[1]), 0, sizeof(double));
                            mappedStream.Write(BitConverter.GetBytes(global.controller_position[2]), 0, sizeof(double));
                        }
                        else
                        {
                            mappedStream.Seek(sizeof(double) * 3, SeekOrigin.Current);
                        }

                        UInt32 newOverrideButtons = global.override_application_menu ? ALVR_FREEPIE_BUTTON_APPLICATION_MENU : 0;
                        UInt32 newButtons = global.application_menu ? ALVR_FREEPIE_BUTTON_APPLICATION_MENU : 0;

                        mappedStream.Write(BitConverter.GetBytes(newOverrideButtons), 0, sizeof(UInt32));
                        mappedStream.Write(BitConverter.GetBytes(newOverrideButtons), 0, sizeof(UInt32));
                    }
                }
                finally
                {
                    mutex.ReleaseMutex();
                }
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

        static readonly UInt32 ALVR_FREEPIE_SIGNATURE_V1 = 0x11223344;

        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION = 1 << 0;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION = 1 << 1;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION = 1 << 2;
        static readonly UInt32 ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION = 1 << 3;

        static readonly UInt32 ALVR_FREEPIE_BUTTON_APPLICATION_MENU = 1 << 0;

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

        public bool override_application_menu;
        public bool application_menu;
        public bool override_head_orientation;
        public bool override_head_position;
        public bool override_controller_orientation;
        public bool override_controller_position;

        // yaw pitch roll
        public double[] head_orientation = new double[3];
        // x y z
        public double[] head_position = new double[3];
        // yaw pitch roll
        public double[] controller_orientation = new double[3];
        // x y z
        public double[] controller_position = new double[3];
    }
}
