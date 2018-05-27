using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Diagnostics;
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
        TcpClient client;
        enum ServerStatus
        {
            CONNECTING,
            CONNECTED,
            DEAD
        };
        ServerStatus status = ServerStatus.DEAD;
        string buf = "";

        public Launcher()
        {
            InitializeComponent();
        }

        async private Task<string> SendCommand(string command)
        {
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

        async private Task<string> ReadNextMessage()
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
                status = ServerStatus.DEAD;
                UpdateServerStatus();
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

        async private void button2_Click(object sender, EventArgs e)
        {
            await SendCommand("EnableTestMode " + metroTextBox1.Text);
        }

        async private void button3_Click(object sender, EventArgs e)
        {
            SendCommand("EnableDriverTestMode " + metroTextBox2.Text);
        }

        private void Launcher_Load(object sender, EventArgs e)
        {
            System.Reflection.Assembly assembly = System.Reflection.Assembly.GetExecutingAssembly();
            FileVersionInfo fvi = FileVersionInfo.GetVersionInfo(assembly.Location);
            string version = fvi.FileVersion;
            var split = version.Split('.');
            versionLabel.Text = "v" + split[0] + "." + split[1];

            metroTabControl1.SelectedTab = serverTab;

            UpdateServerStatus();

            Connect();
        }

        async private void Connect()
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
            }

            Invoke((MethodInvoker)(() =>
            {
                metroProgressSpinner1.Hide();
                if (client.Connected)
                {
                    status = ServerStatus.CONNECTED;
                    UpdateServerStatus();
                    UpdateClients();
                }
                else
                {
                    status = ServerStatus.DEAD;
                    UpdateServerStatus();
                }
            }));
        }

        private void UpdateServerStatus()
        {
            if (status == ServerStatus.CONNECTING)
            {
                metroLabel3.Text = "Checking...";
                metroLabel3.BackColor = Color.White;
                metroLabel3.ForeColor = Color.Black;
                startServerButton.Hide();
            }
            else if (status == ServerStatus.CONNECTED)
            {
                metroLabel3.Text = "Server is alive!";
                metroLabel3.BackColor = Color.LimeGreen;
                metroLabel3.ForeColor = Color.White;
                startServerButton.Hide();
            }
            else if (status == ServerStatus.DEAD)
            {
                metroLabel3.Text = "Server is down";
                metroLabel3.BackColor = Color.Gray;
                metroLabel3.ForeColor = Color.White;
                startServerButton.Show();
            }
        }

        async private void metroButton4_Click(object sender, EventArgs e)
        {
            string str = await SendCommand("GetConfig");
            logText.Text = str.Replace("\n", "\r\n");
        }

        async private void metroButton5_Click(object sender, EventArgs e)
        {
            await SendCommand("SetConfig DebugFrameIndex " + (metroCheckBox1.Checked ? "1" : "0"));
        }

        async private void metroCheckBox2_CheckedChanged(object sender, EventArgs e)
        {
            await SendCommand("Suspend " + (metroCheckBox2.Checked ? "1" : "0"));
        }

        async private void metroCheckBox3_CheckedChanged(object sender, EventArgs e)
        {
            await SendCommand("SetConfig UseKeyedMutex " + (metroCheckBox2.Checked ? "1" : "0"));
        }

        private void metroButton6_Click(object sender, EventArgs e)
        {
            // Check existence of vrmonitor.
            Microsoft.Win32.RegistryKey regkey =
                Microsoft.Win32.Registry.ClassesRoot.OpenSubKey(@"vrmonitor", false);
            if (regkey == null)
            {
                MessageBox.Show("SteamVR is not installed. Please install and retry.");
            }
            else
            {
                Process.Start("vrmonitor:");
            }
        }

        async private void UpdateClients()
        {
            if (!client.Connected)
            {
                Connect();
                return;
            }
            string str = await SendCommand("GetConfig");
            logText.Text = str.Replace("\n", "\r\n");

            if (str.Contains("Connected 1\n")){
                // Connected
                runningPanel.Show();
                findingPanel.Hide();
                return;
            }
            runningPanel.Hide();
            findingPanel.Show();

            str = await SendCommand("GetRequests");

            foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
            {
                // Mark as old data
                row.Tag = 0;
            }

            foreach (var s in str.Split('\n'))
            {
                if (s == "")
                {
                    continue;
                }
                var elem = s.Split(" ".ToCharArray(), 2);

                bool found = false;
                foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
                {
                    if ((string)row.Cells[1].Value == elem[0])
                    {
                        found = true;

                        row.Cells[0].Value = elem[1];
                        // Mark as new data
                        row.Tag = 1;
                    }
                }
                if (!found)
                {
                    int index = dataGridView1.Rows.Add(new string[] { elem[1], elem[0], "Connect" });
                    dataGridView1.Rows[index].Tag = 1;
                }
            }
            for (int j = dataGridView1.Rows.Count - 1; j >= 0; j--)
            {
                // Remove old row
                if ((int)dataGridView1.Rows[j].Tag == 0)
                {
                    dataGridView1.Rows.RemoveAt(j);
                }
            }
            
        }

        private void timer1_Tick(object sender, EventArgs e)
        {
            UpdateClients();
        }

        async private void dataGridView1_CellContentClick(object sender, DataGridViewCellEventArgs e)
        {
            if (dataGridView1.Columns[e.ColumnIndex].Name == "Button")
            {
                string IPAddr = (string)dataGridView1.Rows[e.RowIndex].Cells[1].Value;
                await SendCommand("Connect " + IPAddr);
            }
        }

        async private void metroButton3_Click(object sender, EventArgs e)
        {
            await SendCommand("Capture");
        }

        async private void sendDebugPos_Click(object sender, EventArgs e)
        {
            await SendCommand("SetDebugPos " + (debugPosCheckBox.Checked ? "1" : "0") + " " + debugXTextBox.Text + " " + debugYTextBox.Text + " " + debugZTextBox);
        }
    }
}
