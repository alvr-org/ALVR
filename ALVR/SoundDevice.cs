using Codeplex.Data;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    class DeviceQuery
    {
        [DllImport("kernel32.dll")]
        public static extern IntPtr LoadLibrary(string dllToLoad);

        [DllImport("kernel32.dll")]
        public static extern IntPtr GetProcAddress(IntPtr hModule, string procedureName);

        [DllImport("kernel32.dll")]
        public static extern bool FreeLibrary(IntPtr hModule);

        [DllImport("kernel32.dll")]
        public static extern bool SetDllDirectory(string path);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void GetSoundDevices(out IntPtr buf, out int len);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void ReleaseBuffer(IntPtr buf);

        public class SoundDevice
        {
            public string id;
            public string name;
            public bool isDefault;
        }

        static public List<SoundDevice> GetSoundDeviceList()
        {
            SetDllDirectory(Utils.GetDllDirectory(Utils.GetDriverPath()));

            IntPtr pDll = LoadLibrary(Utils.GetDllPath(Utils.GetDriverPath()));
            if (pDll == IntPtr.Zero)
            {
                int err = Marshal.GetLastWin32Error();
                MessageBox.Show("Cannot load library. ALVR works only on 64bits Windows with NVIDIA GPU.\r\n" + Utils.GetDllPath(Utils.GetDriverPath()) + "\r\nCode:" + err);
                throw new Exception();
            }

            IntPtr GetSoundDevicesAddr = GetProcAddress(pDll, "GetSoundDevices");
            IntPtr ReleaseBufferAddr = GetProcAddress(pDll, "ReleaseBuffer");
            if (GetSoundDevicesAddr == IntPtr.Zero || ReleaseBufferAddr == IntPtr.Zero)
            {
                MessageBox.Show("Cannot find function from \r\n" + Utils.GetDllPath(Utils.GetDriverPath()));
                throw new Exception();
            }
            var GetSoundDevicesFunc = (GetSoundDevices)Marshal.GetDelegateForFunctionPointer(
                                                                                    GetSoundDevicesAddr,
                                                                                    typeof(GetSoundDevices));
            var ReleaseBufferFunc = (ReleaseBuffer)Marshal.GetDelegateForFunctionPointer(
                                                                                    ReleaseBufferAddr,
                                                                                    typeof(ReleaseBuffer));
            IntPtr ptr;
            int len;
            GetSoundDevicesFunc(out ptr, out len);
            string buf = Marshal.PtrToStringUni(ptr, len);
            ReleaseBufferFunc(ptr);

            var json = DynamicJson.Parse(buf, Encoding.UTF8);

            var deviceList = new List<SoundDevice>();
            foreach (var elem in json)
            {
                var desc = new SoundDevice();
                desc.name = elem.name;
                desc.id = elem.id;
                desc.isDefault = elem.is_default;
                deviceList.Add(desc);
            }

            bool result = FreeLibrary(pDll);

            return deviceList;
        }

    }
}
