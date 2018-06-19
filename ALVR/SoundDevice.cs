using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    class SoundDevice
    {
        [DllImport("kernel32.dll")]
        public static extern IntPtr LoadLibrary(string dllToLoad);

        [DllImport("kernel32.dll")]
        public static extern IntPtr GetProcAddress(IntPtr hModule, string procedureName);

        [DllImport("kernel32.dll")]
        public static extern bool FreeLibrary(IntPtr hModule);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void GetSoundDevices(out IntPtr buf, out int len);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void ReleaseSoundDeviesBuffer(IntPtr buf);

        static public List<string> GetSoundDeviceList()
        {
            IntPtr pDll = LoadLibrary(Utils.GetDllPath(Utils.GetDriverPath()));
            if (pDll == IntPtr.Zero)
            {
                int err = Marshal.GetLastWin32Error();
                MessageBox.Show("Cannot load library. ALVR works only on 64bits Windows.\r\n" + Utils.GetDllPath(Utils.GetDriverPath()) + "\r\nCode:" + err);
                throw new Exception();
            }

            IntPtr GetSoundDevicesAddr = GetProcAddress(pDll, "GetSoundDevices");
            IntPtr ReleaseSoundDeviesBufferAddr = GetProcAddress(pDll, "ReleaseSoundDeviesBuffer");
            if (GetSoundDevicesAddr == IntPtr.Zero || ReleaseSoundDeviesBufferAddr == IntPtr.Zero)
            {
                MessageBox.Show("Cannot find function from \r\n" + Utils.GetDllPath(Utils.GetDriverPath()));
                throw new Exception();
            }
            var GetSoundDevicesFunc = (GetSoundDevices)Marshal.GetDelegateForFunctionPointer(
                                                                                    GetSoundDevicesAddr,
                                                                                    typeof(GetSoundDevices));
            var ReleaseSoundDeviesBufferFunc = (ReleaseSoundDeviesBuffer)Marshal.GetDelegateForFunctionPointer(
                                                                                    ReleaseSoundDeviesBufferAddr,
                                                                                    typeof(ReleaseSoundDeviesBuffer));
            IntPtr ptr;
            int len;
            GetSoundDevicesFunc(out ptr, out len);
            string buf = Marshal.PtrToStringUni(ptr, len);
            ReleaseSoundDeviesBufferFunc(ptr);

            List<string> deviceList = buf.Split('\0').ToList();
            deviceList.RemoveAt(deviceList.Count - 1);

            bool result = FreeLibrary(pDll);

            return deviceList;
        }

    }
}
