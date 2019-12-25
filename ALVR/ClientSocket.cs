using Codeplex.Data;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace ALVR
{
    class ClientSocket
    {
        TcpClient client;
        Action StartServerCallback;
        Action ConnectionClosedCallback;
        int RequestId = 1;

        public ClientSocket(Action startServerCallback, Action connectionClosedCallback)
        {
            StartServerCallback = startServerCallback;
            ConnectionClosedCallback = connectionClosedCallback;
        }

        async public Task<bool> Connect(string host, int port)
        {
            try
            {
                client = new TcpClient();
                await client.ConnectAsync(host, port);
            }
            catch (Exception) {
                return false;
            }

            if(Properties.Settings.Default.steamAutolaunch) {
                StartServerCallback();
            }
           

            Task t = ReadLoop();
            Task k = KeepAliveLoop();
            return true;
        }

        async public Task Disconnect()
        {
            if (client != null)
            {
                await SendCommand("Close");
                client.Close();
            }
            client = null;
        }

        async private Task ReadLoop()
        {
            try
            {
                while (true)
                {
                    dynamic message = await ReadNextMessage();
                    switch (message.command)
                    {
                        case "StartServer":
                            StartServerCallback();
                            await ReplyMessage((int)message.requestId, "OK");
                            break;
                        case "Ping":
                            await ReplyMessage((int)message.requestId, "Pong");
                            break;
                        case "Pong":
                            break;
                        case "Close":
                            return;
                    }
                }
            }
            catch (Exception e)
            {
                System.Console.WriteLine(e);
            }
            finally
            {
                ConnectionClosedCallback();
                if (client != null)
                {
                    client.Close();
                }
                client = null;
            }
        }

        async private Task KeepAliveLoop()
        {
            try
            {
                while (client != null)
                {
                    var t = SendCommand("Ping");
                    await Task.Delay(1000);
                    await t;
                }
            }
            catch (Exception)
            {
            }
        }

        async private Task<dynamic> ReadNextMessage()
        {
            byte[] buffer = new byte[4];
            int ret = -1;
            ret = await client.GetStream().ReadAsync(buffer, 0, 4);
            if (ret == 0 || ret < 0)
            {
                // Disconnected
                throw new IOException();
            }
            int length = buffer[0]
                | (buffer[1] << 8) | (buffer[2] << 16) | (buffer[3] << 24);
            if (length == 0 || length > 1000 * 1000 * 10)
            {
                throw new IOException();
            }
            buffer = new byte[length];

            ret = await client.GetStream().ReadAsync(buffer, 0, length);
            if (ret == 0 || ret < 0)
            {
                // Disconnected
                throw new IOException();
            }
            else
            {
                return DynamicJson.Parse(Encoding.UTF8.GetString(buffer));
            }
        }

        async private Task SendMessage(byte[] buffer)
        {
            if (client == null)
            {
                return;
            }
            int length = buffer.Length;
            byte[] packet = new byte[4 + buffer.Length];
            packet[0] = (byte)length;
            packet[1] = (byte)(length >> 8);
            packet[2] = (byte)(length >> 16);
            packet[3] = (byte)(length >> 24);

            Array.Copy(buffer, 0, packet, 4, buffer.Length);

            await client.GetStream().WriteAsync(packet, 0, packet.Length);
        }

        async private Task ReplyMessage(int requestId, string s)
        {
            string str = DynamicJson.Serialize(new
            {
                requestId = requestId,
                result = s
            });
            await SendMessage(Encoding.UTF8.GetBytes(str));
        }

        async private Task SendCommand(string s)
        {
            string str = DynamicJson.Serialize(new
            {
                requestId = RequestId++,
                command = s
            });
            await SendMessage(Encoding.UTF8.GetBytes(str));
        }
    }
}
