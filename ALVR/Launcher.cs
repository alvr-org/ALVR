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
using MetroFramework.Forms;

namespace RemoteGlassLauncher
{
    public partial class Launcher : MetroFramework.Forms.MetroForm
    {
        string m_Host = "127.0.0.1";
        int m_Port = 9944;
        TcpClient client = new TcpClient();

        public Launcher()
        {
            InitializeComponent();
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
            SendCommand("EnableTestMode " + metroTextBox1.Text);
        }

        private void button3_Click(object sender, EventArgs e)
        {
            SendCommand("EnableDriverTestMode " + metroTextBox2.Text);
        }

        private void Launcher_Load(object sender, EventArgs e)
        {

            metroLabel3.Text = "Checking server state...";
            metroLabel3.BackColor = Color.White;
            metroLabel3.ForeColor = Color.Black;

            client.BeginConnect(m_Host, m_Port, (e2) => {
                bool success = false;

                try
                {
                    client.EndConnect(e2);
                    success = true;
                }
                catch (SocketException e1)
                {
                    success = false;
                }

                Invoke((MethodInvoker)(() =>
                {
                    metroProgressSpinner1.Hide();
                    if (success)
                    {
                        metroLabel3.Text = "Server is alive!";
                        metroLabel3.BackColor = Color.LimeGreen;
                        metroLabel3.ForeColor = Color.White;
                    }
                    else
                    {
                        metroLabel3.Text = "Server is down";
                        metroLabel3.BackColor = Color.Gray;
                        metroLabel3.ForeColor = Color.White;
                    }
                }));
            }, null);

        }
    }
}
