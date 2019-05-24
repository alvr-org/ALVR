using Codeplex.Data;
using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace ALVR
{
    class ClientList : IEnumerable<DeviceDescriptor>
    {
        HelloListener helloListener;
        List<DeviceDescriptor> autoConnectList = new List<DeviceDescriptor>();
        List<DeviceDescriptor> clients = new List<DeviceDescriptor>();
        public bool EnableAutoConnect { get; set; } = true;

        public ClientList(string serialized)
        {
            helloListener = new HelloListener(NewClientCallback);
            try
            {
                var json = DynamicJson.Parse(serialized);
                foreach (var d in json) {
                    var newobj = new DeviceDescriptor();
                    if (d.DeviceName == null)
                    {
                        continue;
                    }
                    newobj.DeviceName = d.DeviceName;
                    newobj.ClientHost = d.ClientHost;
                    newobj.ClientPort = (int)d.ClientPort;
                    newobj.Version = (UInt32)d.Version;
                    newobj.RefreshRates = (byte[])d.RefreshRates;
                    newobj.RenderWidth = (UInt16)d.RenderWidth;
                    newobj.RenderHeight = (UInt16)d.RenderHeight;
                    newobj.EyeFov = d.EyeFov;
                    newobj.DeviceType = (byte)d.DeviceType;
                    newobj.DeviceSubType = (byte)d.DeviceSubType;
                    newobj.DeviceCapabilityFlags = (UInt32)d.DeviceCapabilityFlags;
                    newobj.ControllerCapabilityFlags = (UInt32)d.ControllerCapabilityFlags;
                    newobj.Online = false;
                    autoConnectList.Add(newobj);
                    clients.Add(newobj);
                }
            }
            catch (Exception)
            {
                autoConnectList.Clear();
            }
        }

        public string Serialize()
        {
            return DynamicJson.Serialize(autoConnectList);
        }

        public void AddAutoConnect(DeviceDescriptor descriptor)
        {
            if (!autoConnectList.Contains(descriptor))
            {
                autoConnectList.Add(descriptor);
            }
        }

        public void RemoveAutoConnect(DeviceDescriptor descriptor)
        {
            autoConnectList.Remove(descriptor);
        }

        public DeviceDescriptor GetAutoConnectableClient()
        {
            var list = autoConnectList.Where(x =>
            {
                return clients.Find(y => x.Equals(y) && y.VersionOk && y.Online) != null;
            });
            if (list.Count() != 0)
            {
                if (!EnableAutoConnect)
                {
                    return null;
                }

                return list.First();
            }
            return null;
        }

        public bool InAutoConnectList(DeviceDescriptor client)
        {
            return autoConnectList.Contains(client);
        }

        public void StartListening()
        {
            helloListener.Start();
        }

        private void NewClientCallback(DeviceDescriptor descriptor)
        {
            if (clients.Contains(descriptor))
            {
                var found = clients.FindIndex((d) => d.Equals(descriptor));
                clients[found] = descriptor;
                found = autoConnectList.FindIndex((d) => d.Equals(descriptor));
                if (found != -1)
                {
                    autoConnectList[found] = descriptor;
                }
            }
            else
            {
                clients.Add(descriptor);
            }
        }

        /// <summary>
        /// Remove aged client entry.
        /// </summary>
        public void Refresh()
        {
            var current = DateTime.Now.Ticks;
            for (int i = 0; i < clients.Count; i++)
            {
                if (TimeSpan.FromTicks(current - clients[i].LastUpdate).TotalSeconds > 5)
                {
                    if (!InAutoConnectList(clients[i]))
                    {
                        clients.RemoveAt(i);
                        i--;
                    }
                    else
                    {
                        clients[i].Online = false;
                    }
                }
            }
        }

        public IEnumerator<DeviceDescriptor> GetEnumerator()
        {
            return ((IEnumerable<DeviceDescriptor>)clients).GetEnumerator();
        }

        IEnumerator IEnumerable.GetEnumerator()
        {
            return ((IEnumerable<DeviceDescriptor>)clients).GetEnumerator();
        }
    }
}
