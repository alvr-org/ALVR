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
using Codeplex.Data;
using MetroFramework.Forms;
using Microsoft.Win32;

namespace ALVR
{
    public partial class Launcher : MetroFramework.Forms.MetroForm
    {
        ControlSocket socket = new ControlSocket();
        ServerConfig config = new ServerConfig();
        ClientList clientList;
        bool previousConnectionState = false;

        class ClientTag
        {
            public bool updated = false;
            public ClientList.Client client;
        }

        public Launcher()
        {
            InitializeComponent();
        }

        private void Launcher_Load(object sender, EventArgs e)
        {
            SetFileVersion();

            var list = SoundDevice.GetSoundDeviceList();
            foreach (var device in list)
            {
                soundDeviceComboBox.Items.Add(device);
            }

            LoadSettings();

            config.Save();

            UpdateEnableControllerState();
            UpdateSoundCheckboxState();

            DriverInstaller.RemoveOtherDriverInstallations();
            CheckDriverInstallStatus();

            metroTabControl1.SelectedTab = serverTab;

            UpdateServerStatus();

            messageLabel.Text = "Checking server status. Please wait...";
            ShowMessagePanel();

            socket.Update();

            SoundDevice.GetSoundDeviceList();

            timer1.Start();
        }

        private void LoadSettings()
        {
            if (Properties.Settings.Default.UpgradeRequired)
            {
                Properties.Settings.Default.Upgrade();
                Properties.Settings.Default.UpgradeRequired = false;
                Properties.Settings.Default.Save();
            }

            resolutionComboBox.DataSource = ServerConfig.supportedResolutions;
            resolutionComboBox.Text = new ServerConfig.Resolution { width = Properties.Settings.Default.renderWidth }.ToString();

            triggerComboBox.DataSource = ServerConfig.supportedButtons.Clone();
            trackpadClickComboBox.DataSource = ServerConfig.supportedButtons;
            triggerComboBox.SelectedIndex = ServerConfig.FindButton(Properties.Settings.Default.controllerTriggerMode);
            trackpadClickComboBox.SelectedIndex = ServerConfig.FindButton(Properties.Settings.Default.controllerTrackpadClickMode);

            recenterButtonComboBox.DataSource = ServerConfig.supportedRecenterButton;
            recenterButtonComboBox.SelectedIndex = Properties.Settings.Default.controllerRecenterButton;

            if (Properties.Settings.Default.soundDevice != "")
            {
                for (int i = 0; i < soundDeviceComboBox.Items.Count; i++)
                {
                    if ((string)soundDeviceComboBox.Items[i] == Properties.Settings.Default.soundDevice)
                    {
                        soundDeviceComboBox.SelectedIndex = i;
                        break;
                    }
                }
            }
            if (soundDeviceComboBox.SelectedIndex == -1 && soundDeviceComboBox.Items.Count > 0)
            {
                soundDeviceComboBox.SelectedIndex = 0;
            }

            clientList = new ClientList(Properties.Settings.Default.autoConnectList);
        }

        private void SaveSettings()
        {
            offsetPosXTextBox.Text = Utils.ParseFloat(offsetPosXTextBox.Text).ToString();
            offsetPosYTextBox.Text = Utils.ParseFloat(offsetPosYTextBox.Text).ToString();
            offsetPosZTextBox.Text = Utils.ParseFloat(offsetPosZTextBox.Text).ToString();

            Properties.Settings.Default.renderWidth = ((ServerConfig.Resolution)resolutionComboBox.SelectedItem).width;
            Properties.Settings.Default.controllerTriggerMode = ((ServerConfig.ComboBoxCustomItem)triggerComboBox.SelectedItem).value;
            Properties.Settings.Default.controllerTrackpadClickMode = ((ServerConfig.ComboBoxCustomItem)trackpadClickComboBox.SelectedItem).value;
            Properties.Settings.Default.controllerRecenterButton = recenterButtonComboBox.SelectedIndex;
            Properties.Settings.Default.autoConnectList = clientList.Serialize();

            if (soundDeviceComboBox.SelectedIndex != -1)
            {
                Properties.Settings.Default.soundDevice = (string)soundDeviceComboBox.SelectedItem;
            }

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
            versionLabel.Text = "v" + fvi.ProductVersion;

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
                UpdateConnectionState(false);
                return;
            }
            string str = await socket.SendCommand("GetConfig");
            if (str == "")
            {
                UpdateConnectionState(false);
                return;
            }
            logText.Text = str.Replace("\n", "\r\n");

            var configs = ParsePacket(str);
            if (configs["Connected"] == "1")
            {
                // Connected
                UpdateConnectionState(true, configs["ClientName"] + " " + configs["Client"] + " " + configs["RefreshRate"]);

                connectedLabel.Text = "Connected!\r\n\r\n" + configs["ClientName"] + "\r\n"
                    + configs["Client"] + "\r\n" + configs["RefreshRate"] + " FPS";

                autoConnectCheckBox.CheckedChanged -= autoConnectCheckBox_CheckedChanged;
                autoConnectCheckBox.Checked = clientList.InAutoConnectList(configs["ClientName"], configs["Client"]);
                autoConnectCheckBox.CheckedChanged += autoConnectCheckBox_CheckedChanged;
                ShowConnectedPanel();

                UpdateClientStatistics();
                return;
            }
            UpdateConnectionState(false);
            ShowFindingPanel();

            var clients = clientList.ParseRequests(await socket.SendCommand("GetRequests"));

            foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
            {
                // Mark as old data
                ((ClientTag)row.Tag).updated = false;
            }

            foreach (var client in clients)
            {
                DataGridViewRow found = null;
                foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
                {
                    ClientTag tag1 = ((ClientTag)row.Tag);
                    if (tag1.client.Equals(client))
                    {
                        found = row;
                        tag1.client = client;
                        tag1.updated = true;
                    }
                }
                if (found == null)
                {
                    int index = dataGridView1.Rows.Add();
                    found = dataGridView1.Rows[index];
                    ClientTag tag2 = new ClientTag();
                    found.Tag = tag2;
                    tag2.client = client;
                    tag2.updated = true;
                }

                Color color = Color.Black;
                if (!client.Online)
                {
                    color = Color.DarkGray;
                }
                found.Cells[0].Style.ForeColor = color;
                found.Cells[0].Style.SelectionForeColor = color;
                found.Cells[1].Style.ForeColor = color;
                found.Cells[1].Style.SelectionForeColor = color;
                found.Cells[2].Style.ForeColor = color;
                found.Cells[2].Style.SelectionForeColor = color;

                found.Cells[0].Value = client.Name;
                found.Cells[1].Value = client.Address;
                found.Cells[2].Value = client.Online ? (client.RefreshRate + " FPS") : "Offline";

                string buttonLabel = "Connect";
                if (!client.Online)
                {
                    buttonLabel = "Remove";
                }
                else if (!client.VersionOk)
                {
                    buttonLabel = "Wrong version";
                }

                if ((string)found.Cells[3].Value != buttonLabel)
                {
                    found.Cells[3].Value = buttonLabel;
                }
            }
            for (int j = dataGridView1.Rows.Count - 1; j >= 0; j--)
            {
                // Remove old row
                ClientTag tag3 = ((ClientTag)dataGridView1.Rows[j].Tag);
                if (!tag3.updated)
                {
                    dataGridView1.Rows.RemoveAt(j);
                }
            }
            noClientLabel.Visible = dataGridView1.Rows.Count == 0;

            var autoConnect = clientList.GetAutoConnectableClient();
            if (autoConnect != null)
            {
                await clientList.Connect(socket, autoConnect);
            }
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

        async private Task SendOffsetPos()
        {
            SaveSettings();
            await socket.SendCommand("SetOffsetPos " + (offsetPosCheckBox.Checked ? "1" : "0") + " " + offsetPosXTextBox.Text + " " + offsetPosYTextBox.Text + " " + offsetPosZTextBox.Text);
        }

        private void UpdateConnectionState(bool connected, string args = "")
        {
            if (!previousConnectionState && connected)
            {
                previousConnectionState = connected;
                RunConnectCommand(args);
            }else if (previousConnectionState && !connected)
            {
                previousConnectionState = connected;
                RunDisconnectCommand();
            }
        }

        private void RunConnectCommand(string args)
        {
            var command = connectCommandTextBox.Text;
            if (command == "")
            {
                return;
            }
            Utils.ExecuteProcess(command, "connect " + args);
        }

        private void RunDisconnectCommand()
        {
            var command = disconnectCommandTextBox.Text;
            if (command == "")
            {
                return;
            }
            Utils.ExecuteProcess(command, "disconnect");
        }

        private void UpdateSoundCheckboxState()
        {
            soundDeviceComboBox.Enabled = soundCheckBox.Checked;
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
                var tag = ((ClientTag)dataGridView1.Rows[e.RowIndex].Tag);
                
                if (!tag.client.Online)
                {
                    // Remove from auto connect list.
                    clientList.RemoveAutoConnect(tag.client);
                    SaveSettings();
                    UpdateClients();
                    return;
                }
                if (!tag.client.VersionOk)
                {
                    MessageBox.Show("Please check the version of client and server and update both.");
                    return;
                }
                // Reenable auto connect.
                clientList.EnableAutoConnect = true;
                await clientList.Connect(socket, tag.client);
            }
        }

        async private void metroButton3_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("Capture");
        }

        async private void sendOffsetPos_Click(object sender, EventArgs e)
        {
            await SendOffsetPos();
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
            // Disable auto connect to avoid immediate auto reconnection.
            clientList.EnableAutoConnect = false;
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

        async private void autoConnectCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            string str = await socket.SendCommand("GetConfig");
            if (str == "")
            {
                return;
            }

            var configs = ParsePacket(str);
            if (!configs.ContainsKey("ClientName") || !configs.ContainsKey("Client"))
            {
                // Not connected?
                return;
            }
            if (autoConnectCheckBox.Checked)
            {
                clientList.AddAutoConnect(configs["ClientName"], configs["Client"]);
            } else
            {
                clientList.RemoveAutoConnect(configs["ClientName"], configs["Client"]);
            }
            SaveSettings();
        }

        private void dataGridView1_SelectionChanged(object sender, EventArgs e)
        {
            dataGridView1.ClearSelection();
        }

        private void refConnectCommandButton_Click(object sender, EventArgs e)
        {
            var result = openFileDialog1.ShowDialog();
            if (result == DialogResult.OK)
            {
                connectCommandTextBox.Text = openFileDialog1.FileName;
            }
        }

        private void refDisconnectCommandButton_Click(object sender, EventArgs e)
        {
            var result = openFileDialog1.ShowDialog();
            if (result == DialogResult.OK)
            {
                disconnectCommandTextBox.Text = openFileDialog1.FileName;
            }
        }

        private void soundCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            UpdateSoundCheckboxState();
        }
    }
}
