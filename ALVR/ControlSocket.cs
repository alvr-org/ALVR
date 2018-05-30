using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;

namespace ALVR
{
    class ControlSocket
    {
        string m_Host = "127.0.0.1";
        int m_Port = 9944;
        TcpClient client;
        string buf = "";

        public enum ServerStatus
        {
            CONNECTING,
            CONNECTED,
            DEAD
        };
        public ServerStatus status { get; private set; } = ServerStatus.DEAD;
        enum ClientStatus
        {
            CONNECTED,
            DEAD
        };
        ClientStatus clientStatus = ClientStatus.DEAD;

        public ControlSocket()
        {
            
        }
        
        async public Task<string> SendCommand(string command)
        {
            if (client == null || !client.Connected)
            {
                return "";
            }
            byte[] buffer = Encoding.UTF8.GetBytes(command + "\n");
            try
            {
                client.GetStream().Write(buffer, 0, buffer.Length);
            }
            catch (Exception e)
            {
            }
            return await ReadNextMessage();
        }

        async public Task<string> ReadNextMessage()
        {
            byte[] buffer = new byte[1000];
            int ret = -1;
            try
            {
                ret = await client.GetStream().ReadAsync(buffer, 0, 1000);
            }
            catch (Exception e)
            {
            }
            if (ret == 0 || ret < 0)
            {
                // Disconnected
                client.Close();
                client = null;
                status = ServerStatus.DEAD;
                return "";
            }
            else
            {
                string str = Encoding.UTF8.GetString(buffer, 0, ret);
                buf += str;

                int i = buf.IndexOf("\nEND\n");
                if (i == -1)
                {
                    return await ReadNextMessage();
                }
                string ret2 = buf.Substring(0, i);
                buf = buf.Substring(i + 5);
                return ret2;
            }
        }

        async public void Connect()
        {
            if (status != ServerStatus.DEAD && client.Connected)
            {
                return;
            }
            try
            {
                status = ServerStatus.CONNECTING;
                client = new TcpClient();
                await client.ConnectAsync(m_Host, m_Port);
            }
            catch (Exception e)
            {
                Debug.WriteLine("Connection error: " + e + "\r\n" + e.Message);
            }

            if (client.Connected)
            {
                status = ServerStatus.CONNECTED;
            }
            else
            {
                status = ServerStatus.DEAD;
            }
        }

        public void Update()
        {
            if (client == null || !client.Connected)
            {
                Connect();
            }
        }

        public bool Connected
        {
            get { return client.Connected; }
        }
    }
}
