using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace RemoteGlassLauncher
{
    public partial class Form1 : Form
    {
        string m_Host = "127.0.0.1";
        int m_Port = 9944;
        TcpClient client = new TcpClient();

        public Form1()
        {
            InitializeComponent();

            checkBox1.Text = "Checking server state...";
            checkBox1.Checked = false;
            client.BeginConnect(m_Host, m_Port, (e)=> {
                bool success = false;

                try
                {
                    client.EndConnect(e);
                    success = true;
                } catch (SocketException e1) {
                    success = false;
                }
                
                Invoke((MethodInvoker)(() =>
                {
                    if (success)
                    {
                        checkBox1.Text = "Server is alive!";
                        checkBox1.Checked = true;
                    }
                    else
                    {
                        checkBox1.Text = "Server is down";
                        checkBox1.Checked = false;
                    }
                }));
            }, null);

        }

        private void SendCommand(string command)
        {
            byte[] buffer = Encoding.UTF8.GetBytes(command + "\n");
            client.GetStream().Write(buffer, 0, buffer.Length);
        }

        private void button1_Click(object sender, EventArgs e)
        {
            SendCommand("Capture");
        }

        private void ReadNextMessage()
        {
            byte[] buffer = new byte[1000];
            client.GetStream().BeginRead(buffer, 0, 1000, (e) => {
                MessageBox.Show("response:" + Encoding.UTF8.GetChars(buffer));
                ReadNextMessage();
            }, null);
        }

        private void button2_Click(object sender, EventArgs e)
        {
            SendCommand("EnableTestMode " + textBox1.Text);
        }

        private void button3_Click(object sender, EventArgs e)
        {
            SendCommand("EnableDriverTestMode " + textBox2.Text);
        }
    }
}
