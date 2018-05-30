using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;
using MetroFramework.Forms;
using Microsoft.Win32;

namespace ALVR
{
    public partial class Launcher : MetroFramework.Forms.MetroForm
    {
        ControlSocket socket = new ControlSocket();
        string buf = "";
        ServerConfig config = new ServerConfig();

        class ComboBoxCustomItem
        {
            public ComboBoxCustomItem(string s, int val)
            {
                text = s;
                value = val;
            }
            private readonly string text;
            private readonly int value;

            public override string ToString()
            {
                return text;
            }
            public int GetValue()
            {
                return value;
            }
        }

        public Launcher()
        {
            InitializeComponent();
        }

        private void Launcher_Load(object sender, EventArgs e)
        {
            SetFileVersion();

            if (!config.Load())
            {
                Application.Exit();
                return;
            }

            foreach (var width in ServerConfig.supportedWidth)
            {
                int i = resolutionComboBox.Items.Add(width + " x " + (width / 2));
                if (config.renderWidth == width)
                {
                    resolutionComboBox.SelectedItem = resolutionComboBox.Items[i];
                }
            }

            for(int i = 0; i < ServerConfig.supportedButtons.Length; i++)
            {
                var item = new ComboBoxCustomItem(ServerConfig.supportedButtons[i], ServerConfig.supportedButtonId[i]);
                int index = triggerComboBox.Items.Add(item);
                if (ServerConfig.supportedButtonId[i] == config.controllerTriggerMode)
                {
                    triggerComboBox.SelectedIndex = index;
                }
                index = trackpadClickComboBox.Items.Add(item);
                if (ServerConfig.supportedButtonId[i] == config.controllerTrackpadClickMode)
                {
                    trackpadClickComboBox.SelectedIndex = index;
                }
            }

            for (int i = 0; i < ServerConfig.supportedRecenterButton.Length; i++)
            {
                var item = new ComboBoxCustomItem(ServerConfig.supportedRecenterButton[i], i);
                int index = recenterButtonComboBox.Items.Add(item);
                if (i == config.controllerRecenterButton)
                {
                    recenterButtonComboBox.SelectedIndex = index;
                }
            }

            bitrateTrackBar.Value = config.bitrate;

            SetBufferSizeBytes(config.bufferSize);

            CheckDriverInstallStatus();

            metroTabControl1.SelectedTab = serverTab;

            UpdateServerStatus();

            messageLabel.Text = "Checking server status. Please wait...";
            messagePanel.Show();
            findingPanel.Hide();

            socket.Update();

            timer1.Start();
        }

        private void LaunchServer()
        {
            if (!DriverInstaller.InstallDriver())
            {
                CheckDriverInstallStatus();
                return;
            }
            CheckDriverInstallStatus();

            if (!SaveConfig())
            {
                return;
            }

            Process.Start("vrmonitor:");
        }

        private bool SaveConfig()
        {
            // Save json
            config.renderWidth = ServerConfig.supportedWidth[resolutionComboBox.SelectedIndex];
            config.bitrate = bitrateTrackBar.Value;
            config.bufferSize = GetBufferSizeKB() * 1000;
            config.controllerTriggerMode = ((ComboBoxCustomItem)triggerComboBox.SelectedItem).GetValue();
            config.controllerTrackpadClickMode = ((ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).GetValue();
            // Currently, we use same assing to click and touch of trackpad.
            config.controllerTrackpadTouchMode = ((ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).GetValue();
            config.controllerRecenterButton = ((ComboBoxCustomItem)recenterButtonComboBox.SelectedItem).GetValue();

            bool debugLog = debugLogCheckBox.Checked;
            if (!config.Save(debugLog))
            {
                Application.Exit();
                return false;
            }
            return true;
        }


        private void SetFileVersion()
        {
            Assembly assembly = Assembly.GetExecutingAssembly();
            FileVersionInfo fvi = FileVersionInfo.GetVersionInfo(assembly.Location);
            string version = fvi.FileVersion;
            var split = version.Split('.');
            versionLabel.Text = "v" + split[0] + "." + split[1];

            licenseTextBox.Text = Properties.Resources.LICENSE;
        }

        private void UpdateServerStatus()
        {
            if (socket.status == ControlSocket.ServerStatus.CONNECTED)
            {
                metroLabel3.Text = "Server is alive!";
                metroLabel3.BackColor = Color.LimeGreen;
                metroLabel3.ForeColor = Color.White;

                startServerButton.Hide();
            }
            else
            {
                metroLabel3.Text = "Server is down";
                metroLabel3.BackColor = Color.Gray;
                metroLabel3.ForeColor = Color.White;

                messageLabel.Text = "Server is not runnning.\r\nPress \"Start Server\"";
                messagePanel.Show();
                findingPanel.Hide();

                startServerButton.Show();
            }
        }

        async private void UpdateClients()
        {
            if (!socket.Connected)
            {
                return;
            }
            string str = await socket.SendCommand("GetConfig");
            logText.Text = str.Replace("\n", "\r\n");

            if (str.Contains("Connected 1\n"))
            {
                // Connected
                messageLabel.Text = "Connected!\r\nPlease enjoy!";
                messagePanel.Show();
                findingPanel.Hide();
                return;
            }
            messagePanel.Hide();
            findingPanel.Show();

            str = await socket.SendCommand("GetRequests");

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
            noClientLabel.Visible = dataGridView1.Rows.Count == 0;
        }

        private void SetBufferSizeBytes(int bufferSizeBytes)
        {
            int kb = bufferSizeBytes / 1000;
            if (kb == 200)
            {
                bufferTrackBar.Value = 5;
            }
            // Map 0 - 100 to 100kB - 2000kB
            bufferTrackBar.Value = (kb - 100) * 100 / 1900;
        }

        private int GetBufferSizeKB()
        {
            if (bufferTrackBar.Value == 5)
            {
                return 200;
            }
            // Map 0 - 100 to 100kB - 2000kB
            return bufferTrackBar.Value * 1900 / 100 + 100;
        }

        private void CheckDriverInstallStatus()
        {
            if (DriverInstaller.CheckInstalled())
            {
                driverLabel.Text = "Driver is installed";
                driverLabel.Style = MetroFramework.MetroColorStyle.Green;
            }
            else
            {
                driverLabel.Text = "Driver is not installed";
                driverLabel.Style = MetroFramework.MetroColorStyle.Red;
            }
        }

        //
        // Event handlers
        //

        private void timer1_Tick(object sender, EventArgs e)
        {
            socket.Update();
            UpdateClients();
            UpdateServerStatus();
        }

        async private void dataGridView1_CellContentClick(object sender, DataGridViewCellEventArgs e)
        {
            if (dataGridView1.Columns[e.ColumnIndex].Name == "Button")
            {
                string IPAddr = (string)dataGridView1.Rows[e.RowIndex].Cells[1].Value;
                await socket.SendCommand("Connect " + IPAddr);
            }
        }

        async private void metroButton3_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("Capture");
        }

        async private void sendDebugPos_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetDebugPos " + (debugPosCheckBox.Checked ? "1" : "0") + " " + debugXTextBox.Text + " " + debugYTextBox.Text + " " + debugZTextBox);
        }

        private void bitrateTrackBar_ValueChanged(object sender, EventArgs e)
        {
            bitrateLabel.Text = bitrateTrackBar.Value + "Mbps";
        }

        async private void button2_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("EnableTestMode " + metroTextBox1.Text);
        }

        async private void button3_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("EnableDriverTestMode " + metroTextBox2.Text);
        }

        async private void metroButton4_Click(object sender, EventArgs e)
        {
            string str = await socket.SendCommand("GetConfig");
            logText.Text = str.Replace("\n", "\r\n");
        }

        async private void metroButton5_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig debugFrameIndex " + (metroCheckBox1.Checked ? "1" : "0"));
        }

        async private void metroCheckBox2_CheckedChanged(object sender, EventArgs e)
        {
            await socket.SendCommand("Suspend " + (metroCheckBox2.Checked ? "1" : "0"));
        }

        async private void metroCheckBox3_CheckedChanged(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig useKeyedMutex " + (metroCheckBox2.Checked ? "1" : "0"));
        }

        private void metroButton6_Click(object sender, EventArgs e)
        {
            LaunchServer();
        }

        private void bufferTrackBar_ValueChanged(object sender, EventArgs e)
        {
            bufferLabel.Text = GetBufferSizeKB() + "kB";
        }

        private void installButton_Click(object sender, EventArgs e)
        {
            DriverInstaller.InstallDriver();

            CheckDriverInstallStatus();
        }

        private void uninstallButton_Click(object sender, EventArgs e)
        {
            DriverInstaller.UninstallDriver();

            CheckDriverInstallStatus();
        }

        async private void triggerComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            int value = ((ComboBoxCustomItem)triggerComboBox.SelectedItem).GetValue();
            await socket.SendCommand("SetConfig controllerTriggerMode " + value);
        }

        async private void trackpadClickComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            int value = ((ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).GetValue();
            await socket.SendCommand("SetConfig controllerTrackpadClickMode " + value);
            await socket.SendCommand("SetConfig controllerTrackpadTouchMode " + value);
        }

        async private void recenterButtonComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            int value = ((ComboBoxCustomItem)recenterButtonComboBox.SelectedItem).GetValue();
            await socket.SendCommand("SetConfig controllerRecenterButton " + value);
        }
    }
}
