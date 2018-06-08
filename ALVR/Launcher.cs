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
        ServerConfig config = new ServerConfig();

        public Launcher()
        {
            InitializeComponent();
        }

        private void Launcher_Load(object sender, EventArgs e)
        {
            SetFileVersion();

            LoadSettings();

            UpdateEnableControllerState();

            CheckDriverInstallStatus();

            metroTabControl1.SelectedTab = serverTab;

            UpdateServerStatus();

            messageLabel.Text = "Checking server status. Please wait...";
            ShowMessagePanel();

            socket.Update();

            timer1.Start();
        }

        private void LoadSettings()
        {
            resolutionComboBox.DataSource = ServerConfig.supportedResolutions;
            resolutionComboBox.Text = new ServerConfig.Resolution { width = Properties.Settings.Default.renderWidth }.ToString();

            triggerComboBox.DataSource = ServerConfig.supportedButtons.Clone();
            trackpadClickComboBox.DataSource = ServerConfig.supportedButtons;
            triggerComboBox.SelectedIndex = ServerConfig.FindButton(Properties.Settings.Default.controllerTriggerMode);
            trackpadClickComboBox.SelectedIndex = ServerConfig.FindButton(Properties.Settings.Default.controllerTrackpadClickMode);

            recenterButtonComboBox.DataSource = ServerConfig.supportedRecenterButton;
            recenterButtonComboBox.SelectedIndex = Properties.Settings.Default.controllerRecenterButton;
        }

        private void SaveSettings()
        {
            Properties.Settings.Default.renderWidth = ((ServerConfig.Resolution)resolutionComboBox.SelectedItem).width;
            Properties.Settings.Default.controllerTriggerMode = ((ServerConfig.ComboBoxCustomItem)triggerComboBox.SelectedItem).value;
            Properties.Settings.Default.controllerTrackpadClickMode = ((ServerConfig.ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).value;
            Properties.Settings.Default.controllerRecenterButton = recenterButtonComboBox.SelectedIndex;
            Properties.Settings.Default.Save();
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
            SaveSettings();

            if (!config.Save())
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
            versionLabel.Text = "v" + split[0] + "." + split[1] + "." + split[2];

            licenseTextBox.Text = Properties.Resources.LICENSE;
        }

        private void ShowMessagePanel()
        {
            connectedPanel.Hide();
            findingPanel.Hide();
            messagePanel.Show();
        }
        private void ShowFindingPanel()
        {
            connectedPanel.Hide();
            findingPanel.Show();
            messagePanel.Hide();
        }

        private void ShowConnectedPanel()
        {
            connectedPanel.Show();
            findingPanel.Hide();
            messagePanel.Hide();
        }
        private void UpdateServerStatus()
        {
            if (socket.status == ControlSocket.ServerStatus.CONNECTED)
            {
                metroLabel3.Text = "Server is alive!";
                metroLabel3.BackColor = Color.LimeGreen;
                metroLabel3.ForeColor = Color.White;

                metroProgressSpinner1.Hide();
                startServerButton.Hide();
            }
            else
            {
                metroLabel3.Text = "Server is down";
                metroLabel3.BackColor = Color.Gray;
                metroLabel3.ForeColor = Color.White;

                messageLabel.Text = "Server is not runnning.\r\nPress \"Start Server\"";
                ShowMessagePanel();

                metroProgressSpinner1.Show();
                startServerButton.Show();
            }
        }

        async private void UpdateClientStatistics()
        {
            string str = await socket.SendCommand("GetStat");
            int i = 0;
            foreach (var line in str.Split("\n".ToCharArray()))
            {
                var elem = line.Split(" ".ToCharArray(), 2);
                if (elem.Length != 2)
                {
                    continue;
                }
                if (statDataGridView.Rows.Count <= i){
                    statDataGridView.Rows.Add(new string[] {  });
                }
                statDataGridView.Rows[i].Cells[0].Value = elem[0];
                statDataGridView.Rows[i].Cells[1].Value = elem[1];

                i++;
            }
        }

        private Dictionary<string, string> ParsePacket(string str)
        {
            Dictionary<string, string> dict = new Dictionary<string, string>();
            foreach (var line in str.Split("\n".ToCharArray()))
            {
                var elem = line.Split(" ".ToCharArray(), 2);
                if (elem.Length != 2)
                {
                    continue;
                }
                dict.Add(elem[0], elem[1]);
            }
            return dict;
        }

        async private void UpdateClients()
        {
            if (!socket.Connected)
            {
                return;
            }
            string str = await socket.SendCommand("GetConfig");
            if (str == "")
            {
                return;
            }
            logText.Text = str.Replace("\n", "\r\n");

            var configs = ParsePacket(str);
            if (configs["Connected"] == "1")
            {
                // Connected
                connectedLabel.Text = "Connected!\r\n\r\n" + configs["ClientName"] + "\r\n"
                    + configs["Client"] + "\r\n" + configs["RefreshRate"] + " FPS";
                ShowConnectedPanel();

                UpdateClientStatistics();
                return;
            }
            ShowFindingPanel();

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
                var elem = s.Split(" ".ToCharArray(), 4);
                if (elem.Length != 4)
                {
                    timer1.Stop();
                    MessageBox.Show("Invalid server response: " + str.Replace("\n", "\r\n"));
                    Application.Exit();
                    continue;
                }
                var address = elem[0];
                var versionOk = elem[1] == "1";
                var refreshRate = int.Parse(elem[2]);
                var name = elem[3];

                bool found = false;
                foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
                {
                    if ((string)row.Cells[1].Value == address)
                    {
                        found = true;

                        row.Cells[0].Value = name;
                        row.Cells[2].Value = refreshRate + " FPS";
                        if (versionOk)
                        {
                            if ((string)row.Cells[3].Value != "Connect") {
                                row.Cells[3].Value = "Connect";
                            }
                        } else
                        {
                            if ((string)row.Cells[3].Value != "Wrong version")
                            {
                                row.Cells[3].Value = "Wrong version";
                            }
                        }
                        // Mark as new data
                        row.Tag = 1;
                    }
                }
                if (!found)
                {
                    int index = dataGridView1.Rows.Add(new string[] { name, address, refreshRate + " FPS", versionOk ? "Connect" : "Wrong version" });
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

        private void UpdateEnableControllerState()
        {
            triggerComboBox.Enabled = enableControllerCheckBox.Checked;
            trackpadClickComboBox.Enabled = enableControllerCheckBox.Checked;
            recenterButtonComboBox.Enabled = enableControllerCheckBox.Checked;
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
                string version = (string)dataGridView1.Rows[e.RowIndex].Cells[3].Value;
                if (version == "Wrong version")
                {
                    MessageBox.Show("Please check the version of client and server and update both.");
                    return;
                }
                await socket.SendCommand("Connect " + IPAddr);
            }
        }

        async private void metroButton3_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("Capture");
        }

        async private void sendDebugPos_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetDebugPos " + (debugPosCheckBox.Checked ? "1" : "0") + " " + debugXTextBox.Text + " " + debugYTextBox.Text + " " + debugZTextBox.Text);
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
            bufferLabel.Text = config.GetBufferSizeKB() + "kB";
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
            int value = ((ServerConfig.ComboBoxCustomItem)triggerComboBox.SelectedItem).value;
            await socket.SendCommand("SetConfig controllerTriggerMode " + value);
        }

        async private void trackpadClickComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            int value = ((ServerConfig.ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).value;
            await socket.SendCommand("SetConfig controllerTrackpadClickMode " + value);
            //await socket.SendCommand("SetConfig controllerTrackpadTouchMode " + value);
        }

        async private void recenterButtonComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            int value = recenterButtonComboBox.SelectedIndex;
            await socket.SendCommand("SetConfig controllerRecenterButton " + value);
        }

        async private void disconnectButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("Disconnect");
        }

        async private void packetlossButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig causePacketLoss 1000");
        }

        private void enableControllerCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            UpdateEnableControllerState();
        }

        private void listDriversButton_Click(object sender, EventArgs e)
        {
            DriverInstaller.ListDrivers();
        }

        private void Launcher_FormClosed(object sender, FormClosedEventArgs e)
        {
            SaveSettings();
        }
    }
}
