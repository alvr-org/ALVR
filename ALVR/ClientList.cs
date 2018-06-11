using Codeplex.Data;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace ALVR
{
    class ClientList
    {
        public class Client : IEquatable<Client>
        {
            public string Name { get; set; }
            public string Address { get; set; }
            public bool VersionOk { get; set; }
            public int RefreshRate { get; set; }
            public bool Online { get; set; }

            public Client() { }
            
            public Client(string clientName, string address, bool versionOk, int refreshRate = 0, bool online = false)
            {
                Name = clientName;
                Address = address;
                VersionOk = versionOk;
                RefreshRate = refreshRate;
                Online = online;
            }

            public bool Equals(Client other)
            {
                if (other == null)
                    return false;

                return Name == other.Name && Address == other.Address;
            }
        }

        List<Client> autoConnectList = new List<Client>();
        List<Client> clients = new List<Client>();

        public ClientList(string serialized)
        {
            try
            {
                autoConnectList.AddRange((Client[])DynamicJson.Parse(serialized));
            }
            catch (Exception e)
            {
                autoConnectList.Clear();
            }
        }

        public string Serialize()
        {
            return DynamicJson.Serialize(autoConnectList);
        }

        public List<Client> ParseRequests(string requests)
        {
            clients.Clear();
            clients.AddRange(autoConnectList);

            foreach (var s in requests.Split('\n'))
            {
                var elem = s.Split(" ".ToCharArray(), 4);
                if (elem.Length != 4)
                {
                    continue;
                }
                var client = new Client(elem[3], elem[0], elem[1] == "1", int.Parse(elem[2]), true);

                if (clients.Contains(client))
                {
                    // Update status.
                    clients.Remove(client);
                }
                clients.Add(client);
            }
            return clients;
        }

        public void AddAutoConnect(string ClientName, string Address)
        {
            var client = new Client(ClientName, Address, true);
            if (!autoConnectList.Contains(client))
            {
                autoConnectList.Add(client);
            }
        }

        public void RemoveAutoConnect(string ClientName, string Address)
        {
            var client = new Client(ClientName, Address, true);
            autoConnectList.Remove(client);
        }

        public void RemoveAutoConnect(Client client)
        {
            autoConnectList.Remove(client);
        }
    }
}
