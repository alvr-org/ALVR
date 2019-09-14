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
        ClientSocket clientSocket;
        DeviceDescriptor currentClient;
        List<DeviceQuery.SoundDevice> soundDevices = new List<DeviceQuery.SoundDevice>();
        int defaultSoundDeviceIndex = 0;
        bool previousConnectionState = false;
        bool loadingSettings = false;
        bool initComponents = true;

        class ClientTag
        {
            public bool updated = false;
            public DeviceDescriptor client;
        }

        public Launcher()
        {
            clientSocket = new ClientSocket(OnClientMessageStartServer, OnClientConnectionClosed);

            InitializeComponent();
            initComponents = false;
        }

        private void Launcher_Load(object sender, EventArgs e)
        {
            // Set version label
            SetFileVersion();

            //
            // Get sound devices
            //

            try
            {
                soundDevices = DeviceQuery.GetSoundDeviceList();
                int i = 0;
                foreach (var device in soundDevices)
                {
                    string text = device.name;
                    if (device.isDefault)
                    {
                        defaultSoundDeviceIndex = i;
                        text = "(Default) " + text;
                    }
                    soundDeviceComboBox.Items.Add(text);
                    i++;
                }
            }
            catch (Exception)
            {
                Application.Exit();
                return;
            }


            //
            // Load config and create memory mapped object
            //

            codecComboBox.Items.AddRange(ServerConfig.supportedCodecs);
            LoadSettings();

            config.Save(null);

         
   

            //
            // Driver check
            //

            try
            {
                DriverInstaller.CheckDriverPath();
                DriverInstaller.RemoveOtherDriverInstallations();
                CheckDriverInstallStatus();
            }
            catch (Exception e2)
            {
                MessageBox.Show("No SteamVR installation found. Please check installation of SteamVR.\r\n" +
                    e2.Message, "ALVR Fatal Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
                Application.Exit();
            }

            //
            // Open server tab
            //

            metroTabControl1.SelectedTab = serverTab;

            //
            // Update UI
            //

            UpdateServerStatus();
            UpdateSoundCheckboxState();



            //add tooltips
            ToolTip toolTip1 = new ToolTip();
            toolTip1.AutoPopDelay = 50000;
            toolTip1.SetToolTip(this.codecHelp, "Used Video codec \nChoose h265 if possible for better visual quality on lower bitrates");
            toolTip1.SetToolTip(this.bitrateHelp, "Bitrate of video streaming. 30Mbps is recommended. \nHigher bitrates result in better image but also higher latency and network traffic ");
            toolTip1.SetToolTip(this.resolutionHelp, "100% results in the native 2880x1600 resolution of the Oculus Quest \n" +
                "Setting the resolution can bring some improvement in visual quality, but is not recommended \n" +
                "A resolution lower than 100% can reduce latency and network perfomance\n" +
                "Resolutions below 100% require the codec to be h264 wo work properly");

            toolTip1.SetToolTip(this.bufferHelp, "Buffer size on client side\n 200kB is recommended.If you experience packet loss, enlarge buffer.");
            toolTip1.SetToolTip(this.ffrHelp, "Technique where the center of the image is rendered in high resolution while the outskirts are rendered in lower resolution\n" +
                "Results in a much lower video resolution that needs to be transmitted over the network. \n" +
                "The smaller video at the same bitrate can preserve more details and lowers the latency at the same time \n" +
                "FFR causes some visual artefacts at the edges of the view that are more or lesse visible depending on the settings and the game");

            toolTip1.SetToolTip(this.ffrStrengthHelp, "Range from 0 - 5\n" +
                "higher value means less detail outside the foveated region and more artefacts \n" +
                "0 = off");

            toolTip1.SetToolTip(this.ffrShapeRatioHelp, "Range from 1.5 to 2 \nAspect ratio of the foveated region");








            socket.Update();

            timer1.Start();

            clientList.StartListening();

            ShowFindingPanel();
            UpdateClients();
        }

        /// <summary>
        /// Load settings and Update UI
        /// </summary>
        private void LoadSettings()
        {
            var c = Properties.Settings.Default;
            // Disable changed listener on controls.
            loadingSettings = true;

            try
            {
                var index = Array.FindIndex(ServerConfig.supportedScales, x => x == c.resolutionScale);
                if (index < 0)
                {
                    index = ServerConfig.DEFAULT_SCALE_INDEX;
                }
                resolutionComboBox.SelectedIndex = index;
                UpdateResolutionLabel();
                

                codecComboBox.SelectedIndex = c.codec;

                if (c.soundDevice != "")
                {
                    for (int i = 0; i < soundDevices.Count; i++)
                    {
                        if (soundDevices[i].id == c.soundDevice)
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

                clientList = new ClientList(Properties.Settings.Default.autoConnectList, OnWrongVersionDetected);
            }
            finally
            {
                loadingSettings = false;
            }
        }

        /// <summary>
        /// Get settings from UI and save it.
        /// </summary>
        private void SaveSettings()
        {
            var c = Properties.Settings.Default;
      
            trackingFrameOffsetTextBox.Text = Utils.ParseInt(trackingFrameOffsetTextBox.Text).ToString();

            if (resolutionComboBox.SelectedIndex != -1)
            {
                c.resolutionScale = ServerConfig.supportedScales[resolutionComboBox.SelectedIndex];
            }
            else
            {
                c.resolutionScale = ServerConfig.supportedScales[ServerConfig.DEFAULT_SCALE_INDEX];

            }
                       

            c.autoConnectList = clientList.Serialize();

            c.codec = codecComboBox.SelectedIndex;
       


            float ffrMean = Utils.ParseFloat(ffrMeanBox.Text);
            if(ffrMean > 5)
            {
                ffrMeanBox.Text = "5";
            } else if(ffrMean < 0)
            {
                ffrMeanBox.Text = "0";
            }

            float ffrShape = Utils.ParseFloat(ffrRatioBox.Text);
            if (ffrShape > 2)
            {
                ffrMeanBox.Text = "2";
            }
            else if (ffrShape < 1.5)
            {
                ffrRatioBox.Text = "1.5";
            }

           

            if (soundDevices.Count > 0)
            {
                if (!defaultSoundDeviceCheckBox.Checked && soundDeviceComboBox.SelectedIndex != -1)
                {
                    c.soundDevice = soundDevices[soundDeviceComboBox.SelectedIndex].id;
                }
                else
                {
                    c.soundDevice = soundDevices[defaultSoundDeviceIndex].id;
                }
            }
            else
            {
                c.enableSound = false;
                c.soundDevice = "";
            }

            c.Save();
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

            if (Properties.Settings.Default.onlySteamVR)
            {
                Utils.LaunchOnlySteamVR();
            }
            else
            {
                Utils.LaunchSteam();
            }
        }

        private bool SaveConfig()
        {
            SaveSettings();

            if (!config.Save(currentClient))
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

                closeServerButton.Text = "Close server";
                closeServerButton.Enabled = true;
                metroProgressSpinner1.Hide();
                startServerButton.Hide();
            }
            else if (socket.status == ControlSocket.ServerStatus.SHUTTINGDOWN)
            {
                metroLabel3.Text = "Shutting down...";
                metroLabel3.BackColor = Color.LimeGreen;
                metroLabel3.ForeColor = Color.White;

                closeServerButton.Text = "Close server";
                closeServerButton.Enabled = false;
                metroProgressSpinner1.Show();
                startServerButton.Hide();
            }
            else
            {
                if (currentClient != null)
                {
                    metroLabel3.Text = "Server is down";
                    metroLabel3.BackColor = Color.Gray;
                    metroLabel3.ForeColor = Color.White;

                    closeServerButton.Text = "Disconnect";
                    closeServerButton.Enabled = true;
                    statDataGridView.Rows.Clear();
                    metroProgressSpinner1.Show();
                    startServerButton.Show();
                }
                else
                {
                    metroLabel3.Text = "";
                    metroLabel3.BackColor = Color.White;
                    metroLabel3.ForeColor = Color.White;
                    closeServerButton.Text = "Disconnect";
                    closeServerButton.Enabled = false;
                    statDataGridView.Rows.Clear();
                    metroProgressSpinner1.Hide();
                    startServerButton.Hide();
                }
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
                if (statDataGridView.Rows.Count <= i)
                {
                    statDataGridView.Rows.Add(new string[] { });
                }
                statDataGridView.Rows[i].Cells[0].Value = elem[0];
                statDataGridView.Rows[i].Cells[1].Value = elem[1];

                i++;
            }
            str = await socket.SendCommand("GetConfig");
            logText.Text = str.Replace("\n", "\r\n");
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
                try
                {
                    dict.Add(elem[0], elem[1]);
                }
                catch (ArgumentException)
                {
                }
            }
            return dict;
        }

        private void UpdateClients()
        {
            clientList.Refresh();

            foreach (var row in dataGridView1.Rows.Cast<DataGridViewRow>())
            {
                // Mark as old data
                ((ClientTag)row.Tag).updated = false;
            }

            foreach (var client in clientList)
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

                found.Cells[0].Value = client.DeviceName;
                found.Cells[1].Value = client.ClientAddr.ToString();
                found.Cells[2].Value = client.Online ? (client.RefreshRates[0] + " FPS") : "Offline";

                string buttonLabel = "Connect";
                if (!client.Online)
                {
                    buttonLabel = "Remove";
                }
                else if (client.Version != HelloListener.ALVR_PROTOCOL_VERSION)
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
                Connect(autoConnect);
            }
        }

        private void Connect(DeviceDescriptor client)
        {
            if (currentClient != null)
            {
                // Ignore when connected.
                return;
            }
            currentClient = client;

            var task = clientSocket.Connect(currentClient.ClientHost, currentClient.ClientPort);

            connectedLabel.Text = "Connected!\r\n\r\n" + currentClient.DeviceName + "\r\n"
                + currentClient.ClientAddr.ToString() + "\r\n"
                + currentClient.RefreshRates[0] + "Hz " + currentClient.DefaultWidth + "x" + currentClient.DefaultHeight;

            autoConnectCheckBox.CheckedChanged -= autoConnectCheckBox_CheckedChanged;
            autoConnectCheckBox.Checked = clientList.InAutoConnectList(currentClient);
            autoConnectCheckBox.CheckedChanged += autoConnectCheckBox_CheckedChanged;

            UpdateResolutionLabel();
            ShowConnectedPanel();
            UpdateServerStatus();

            // To ensure the driver can detect current config (MemoryMappedFile) on startup,
            // when SteamVR is launched by external.
            SaveConfig();
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
  

       

        private void UpdateConnectionState(bool connected, string args = "")
        {
            if (!previousConnectionState && connected)
            {
                previousConnectionState = connected;
                RunConnectCommand(args);
            }
            else if (previousConnectionState && !connected)
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
          
            if (soundDevices.Count == 0)
            {
                soundCheckBox.Hide();
                defaultSoundDeviceCheckBox.Hide();
                soundDeviceComboBox.Hide();
                noSoundDeviceLabel.Show();
            }
            else
            {
                soundCheckBox.Show();
                defaultSoundDeviceCheckBox.Show();
                soundDeviceComboBox.Show();
                noSoundDeviceLabel.Hide();

                defaultSoundDeviceCheckBox.Enabled = soundCheckBox.Checked;
                soundDeviceComboBox.Enabled = !defaultSoundDeviceCheckBox.Checked && soundCheckBox.Checked;
            }
            SaveSettings();
        }

        //
        // Event handlers
        //

        private void timer1_Tick(object sender, EventArgs e)
        {
            socket.Update();
            UpdateClients();
            UpdateServerStatus();
            UpdateClientStatistics();
        }

        private void dataGridView1_CellContentClick(object sender, DataGridViewCellEventArgs e)
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

                Connect(tag.client);
            }
        }

        async private void captureLayerDDSButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig captureLayerDDS 1");
        }

        async private void captureComposedDDSButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig captureComposedDDS 1");
        }
      

        private void bitrateTrackBar_ValueChanged(object sender, EventArgs e)
        {
            bitrateLabel.Text = bitrateTrackBar.Value + "Mbps";
        }

        async private void sendClientDebugFlagsButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetDebugFlags " + clientDebugFlagsTextBox.Text);
        }

        async private void button3_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("EnableDriverTestMode " + driverTestModeTextBox.Text);
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
              

      

        private void disconnectButton_Click(object sender, EventArgs e)
        {
            // Disable auto connect to avoid immediate auto reconnection.
            clientList.EnableAutoConnect = false;
            Task t = clientSocket.Disconnect();
            currentClient = null;
            ShowFindingPanel();
            socket.Shutdown();
            UpdateServerStatus();
        }

        async private void packetlossButton_Click(object sender, EventArgs e)
        {
            await socket.SendCommand("SetConfig causePacketLoss 1000");
        }

        private void listDriversButton_Click(object sender, EventArgs e)
        {
            DriverInstaller.ListDrivers();
        }


        private void Launcher_FormClosed(object sender, FormClosedEventArgs e)
        {
            SaveSettings();
        }

        async private void 
            autoConnectCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            if (autoConnectCheckBox.Checked)
            {
                clientList.AddAutoConnect(currentClient);
            }
            else
            {
                clientList.RemoveAutoConnect(currentClient);
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

        async private void saveTrackingFrameOffsetButton_Click(object sender, EventArgs e)
        {
            trackingFrameOffsetTextBox.Text = Utils.ParseInt(trackingFrameOffsetTextBox.Text).ToString();
            await socket.SendCommand("SetConfig trackingFrameOffset " + trackingFrameOffsetTextBox.Text);
        }

        async private void suppressFrameDropCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            await socket.SendCommand("SetClientConfig frameQueueSize " + config.GetFrameQueueSize(suppressFrameDropCheckBox.Checked));
        }

        private void defaultSoundDeviceCheckBox_CheckedChanged(object sender, EventArgs e)
        {
            UpdateSoundCheckboxState();
        }

        private void resolutionComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            if (loadingSettings || initComponents)
            {
                return;
            }
            Properties.Settings.Default.resolutionScale =ServerConfig.supportedScales[resolutionComboBox.SelectedIndex];
            UpdateResolutionLabel();
            SaveSettings();
        }

        private void UpdateResolutionLabel()
        {
            int width = ServerConfig.DEFAULT_WIDTH;
            int height = ServerConfig.DEFAULT_HEIGHT;
            if (currentClient != null)
            {
                width = currentClient.DefaultWidth;
                height = currentClient.DefaultHeight;
            }
            resolutionLabel.Text = (int)(width * ServerConfig.supportedScales[resolutionComboBox.SelectedIndex] / 100 )
                + "x" + (int)(height * ServerConfig.supportedScales[resolutionComboBox.SelectedIndex] / 100 );
        }

        // Callbacks for ClientSocket

        private void OnClientMessageStartServer()
        {
            LaunchServer();
        }

        private void OnClientConnectionClosed()
        {
            currentClient = null;

            clientList.Clear();

            ShowFindingPanel();
            UpdateServerStatus();
        }

        private void OnWrongVersionDetected()
        {
            wrongVersionLabel.Visible = true;
        }

        async private void metroButton1_Click(object sender, EventArgs e)
        {
              await socket.SendCommand("SetConfig controllerPoseOffset " + controllerPoseOffset.Text);
        }

        private void controllerPoseOffset_Click(object sender, EventArgs e)
        {

        }

        private void metroCheckBox4_CheckedChanged(object sender, EventArgs e)
        {
            if (loadingSettings  || initComponents)
            {
                return;
            }
            SaveSettings();
        }

        private void metroLink1_Click(object sender, EventArgs e)
        {
            System.Diagnostics.Process.Start("https://www.vb-audio.com/Cable/");
        }

        private void videoTab_Click(object sender, EventArgs e)
        {

        }

        private void metroLabel21_Click(object sender, EventArgs e)
        {

        }

        private void ffrMeanBox_Click(object sender, EventArgs e)
        {

        }

        private void metroCheckBox4_CheckedChanged_1(object sender, EventArgs e)
        {
            if (loadingSettings || initComponents)
            {
                return;
            }
            SaveSettings();
        }

        private void codecComboBox_SelectedIndexChanged(object sender, EventArgs e)
        {
            if (loadingSettings || initComponents)
            {
                return;
            }
            SaveSettings();
        }
    }
}
