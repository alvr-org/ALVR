using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Text;
using System.Threading.Tasks;

namespace ALVR
{
    class DeviceDescriptor : IEquatable<DeviceDescriptor>
    {
        public DeviceDescriptor() { }
        public UInt32 Version { get; set; }
        public string DeviceName { get; set; }
        public byte[] RefreshRates { get; set; } = new byte[4];
        // RenderWidth is reported by client.
        // In Daydream this value means maximum resolution for client.
        // In OculusMobile this value means recommended resoluton for client.
        public UInt16 RenderWidth { get; set; }
        public UInt16 RenderHeight { get; set; }
        // Recommended resolution
        public int DefaultWidth { get { return (int)(RecommendedScale * RenderWidth); } set { } }
        public int DefaultHeight { get { return (int)(RecommendedScale * RenderHeight); } set { } }
        public double[] EyeFov { get; set; }
        public byte DeviceType { get; set; }
        public byte DeviceSubType { get; set; }
        public UInt32 DeviceCapabilityFlags { get; set; }
        public UInt32 ControllerCapabilityFlags { get; set; }

        public double RecommendedScale
        {
            get
            {
                switch (DeviceType)
                {
                    case HelloListener.ALVR_DEVICE_TYPE_OCULUS_MOBILE:
                        return 1.0;
                    case HelloListener.ALVR_DEVICE_TYPE_DAYDREAM:
                        return 0.75;
                    case HelloListener.ALVR_DEVICE_TYPE_CARDBOARD:
                        return 0.75;
                    default:
                        return 1.0;
                }
            }
            set { }
        }

        public string ClientHost { get; set; }
        public int ClientPort { get; set; }
        public string ClientAddr { get { return ClientHost + ":" + ClientPort; } set { } }
        public long LastUpdate { get; set; }
        public bool Online { get; set; }

        public bool VersionOk
        {
            get
            {
                return Version == HelloListener.ALVR_PROTOCOL_VERSION;
            }
            set { }
        }

        public bool HasTouchController
        {
            get
            {
                return DeviceType == HelloListener.ALVR_DEVICE_TYPE_OCULUS_MOBILE &&
                    DeviceSubType == HelloListener.ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_QUEST;
            }
            set { }
        }

        public bool Equals(DeviceDescriptor a)
        {
            return ClientAddr.Equals(a.ClientAddr) && DeviceName.Equals(a.DeviceName);
        }
    }
}
