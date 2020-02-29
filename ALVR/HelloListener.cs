using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;

namespace ALVR
{
    /// <summary>
    /// Listen broadcast hello message from client.
    /// </summary>
    class HelloListener
    {
        // Use different port than 9944 used by server.
        public const int PORT = 9943;
        public const int ALVR_PROTOCOL_VERSION = 24;
        public const int ALVR_PACKET_TYPE_HELLO_MESSAGE = 1;
        public const byte ALVR_DEVICE_TYPE_OCULUS_MOBILE = 1;
        public const byte ALVR_DEVICE_TYPE_DAYDREAM = 2;
        public const byte ALVR_DEVICE_TYPE_CARDBOARD = 3;
        public const byte ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GEARVR = 1;
        public const byte ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GO = 2;
        public const byte ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_QUEST = 3;
        public const byte ALVR_DEVICE_SUBTYPE_DAYDREAM_GENERIC = 1;
        public const byte ALVR_DEVICE_SUBTYPE_DAYDREAM_MIRAGE_SOLO = 2;
        public const byte ALVR_DEVICE_SUBTYPE_CARDBOARD_GENERIC = 1;

        Action<DeviceDescriptor> Callback;
        Action DetectWrongVersionCallback;

        public HelloListener(Action<DeviceDescriptor> callback, Action detectWrongVersionCallback)
        {
            Callback = callback;
            DetectWrongVersionCallback = detectWrongVersionCallback;
        }

        async public void Start()
        {
            var client = new UdpClient(PORT);

            while (true)
            {
                var result = await client.ReceiveAsync();
                var buffer = result.Buffer;

                System.Console.WriteLine("From: " + result.RemoteEndPoint.ToString());
                StringBuilder sb = new StringBuilder();
                for (int i = 0; i < buffer.Length; i++)
                {
                    sb.Append(buffer[i].ToString("X2") + " ");
                    if (i % 16 == 15)
                    {
                        sb.Append("\n");
                    }
                }
                System.Console.WriteLine("Packet:\n" + sb.ToString());

                var descriptor = ParseHelloPacket(buffer);
                if (descriptor != null)
                {
                    AddNewDevice(descriptor, result.RemoteEndPoint);
                }
            }
        }

        private DeviceDescriptor ParseHelloPacket(byte[] buffer)
        {
            try
            {
                DeviceDescriptor descriptor = new DeviceDescriptor();
                int pos = 0;

                // Check packet type
                UInt32 type = ReadUInt32(buffer, ref pos);
                if (type != ALVR_PACKET_TYPE_HELLO_MESSAGE)
                {
                    return null;
                }

                // Verify signature
                if (buffer[pos] != 'A' || buffer[pos + 1] != 'L' ||
                    buffer[pos + 2] != 'V' || buffer[pos + 3] != 'R')
                {
                    return null;
                }
                pos += 4;

                // Check protocol version
                descriptor.Version = ReadUInt32(buffer, ref pos);
                if (descriptor.Version != ALVR_PROTOCOL_VERSION)
                {
                    DetectWrongVersionCallback();
                    return null;
                }
                descriptor.DeviceName = ReadDeviceName(buffer, ref pos);

                descriptor.RefreshRates[0] = buffer[pos++];
                descriptor.RefreshRates[1] = buffer[pos++];
                descriptor.RefreshRates[2] = buffer[pos++];
                descriptor.RefreshRates[3] = buffer[pos++];

                descriptor.RenderWidth = ReadUInt16(buffer, ref pos);
                descriptor.RenderHeight = ReadUInt16(buffer, ref pos);

                // Read fov as float array of 8 elements.
                // [l,r,t,b]*2
                double[] fov = new double[8];
                for (int i = 0; i < 8; i++)
                {
                    fov[i] = BitConverter.ToSingle(buffer, pos);
                    pos += 4;
                }
                descriptor.EyeFov = fov;

                descriptor.DeviceType = buffer[pos++];
                descriptor.DeviceSubType = buffer[pos++];
                descriptor.DeviceCapabilityFlags = ReadUInt32(buffer, ref pos);
                descriptor.ControllerCapabilityFlags = ReadUInt32(buffer, ref pos);

                return descriptor;
            }
            catch (IndexOutOfRangeException)
            {
            }
            return null;
        }

        private UInt16 ReadUInt16(byte[] buffer, ref int pos)
        {
            UInt16 ret = BitConverter.ToUInt16(buffer, pos);
            pos += 2;
            return ret;
        }

        private UInt32 ReadUInt32(byte[] buffer, ref int pos)
        {
            UInt32 ret = BitConverter.ToUInt32(buffer, pos);
            pos += 4;
            return ret;
        }

        private string ReadDeviceName(byte[] buffer, ref int pos)
        {
            string ret = System.Text.Encoding.UTF8.GetString(buffer, pos, 32);
            pos += 32;
            return Regex.Replace(ret.TrimEnd('\0'), "[^a-zA-Z0-9_-]*", "");
        }

        private void AddNewDevice(DeviceDescriptor descriptor, IPEndPoint remoteEndPoint)
        {
            descriptor.LastUpdate = DateTime.Now.Ticks;
            descriptor.ClientHost = remoteEndPoint.Address.ToString();
            descriptor.ClientPort = remoteEndPoint.Port;
            descriptor.Online = true;
            Callback.Invoke(descriptor);
        }

    }
}
