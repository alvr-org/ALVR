namespace RemoteGlassLauncher
{
    partial class Launcher
    {
        /// <summary>
        /// 必要なデザイナー変数です。
        /// </summary>
        private System.ComponentModel.IContainer components = null;

        /// <summary>
        /// 使用中のリソースをすべてクリーンアップします。
        /// </summary>
        /// <param name="disposing">マネージ リソースを破棄する場合は true を指定し、その他の場合は false を指定します。</param>
        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null))
            {
                components.Dispose();
            }
            base.Dispose(disposing);
        }

        #region Windows フォーム デザイナーで生成されたコード

        /// <summary>
        /// デザイナー サポートに必要なメソッドです。このメソッドの内容を
        /// コード エディターで変更しないでください。
        /// </summary>
        private void InitializeComponent()
        {
            this.components = new System.ComponentModel.Container();
            this.metroButton1 = new MetroFramework.Controls.MetroButton();
            this.metroButton2 = new MetroFramework.Controls.MetroButton();
            this.metroProgressSpinner1 = new MetroFramework.Controls.MetroProgressSpinner();
            this.metroButton3 = new MetroFramework.Controls.MetroButton();
            this.metroTabControl1 = new MetroFramework.Controls.MetroTabControl();
            this.serverTab = new MetroFramework.Controls.MetroTabPage();
            this.findingPanel = new MetroFramework.Controls.MetroPanel();
            this.dataGridView1 = new System.Windows.Forms.DataGridView();
            this.NameColumn = new System.Windows.Forms.DataGridViewTextBoxColumn();
            this.AddressColumn = new System.Windows.Forms.DataGridViewTextBoxColumn();
            this.Button = new System.Windows.Forms.DataGridViewButtonColumn();
            this.metroProgressSpinner2 = new MetroFramework.Controls.MetroProgressSpinner();
            this.metroLabel4 = new MetroFramework.Controls.MetroLabel();
            this.runningPanel = new MetroFramework.Controls.MetroPanel();
            this.runningLabel = new MetroFramework.Controls.MetroLabel();
            this.debugTab = new MetroFramework.Controls.MetroTabPage();
            this.metroLabel5 = new MetroFramework.Controls.MetroLabel();
            this.logText = new MetroFramework.Controls.MetroTextBox();
            this.metroCheckBox3 = new MetroFramework.Controls.MetroCheckBox();
            this.metroCheckBox2 = new MetroFramework.Controls.MetroCheckBox();
            this.metroCheckBox1 = new MetroFramework.Controls.MetroCheckBox();
            this.metroTextBox2 = new MetroFramework.Controls.MetroTextBox();
            this.metroTextBox1 = new MetroFramework.Controls.MetroTextBox();
            this.metroLabel2 = new MetroFramework.Controls.MetroLabel();
            this.metroLabel1 = new MetroFramework.Controls.MetroLabel();
            this.metroButton5 = new MetroFramework.Controls.MetroButton();
            this.metroButton4 = new MetroFramework.Controls.MetroButton();
            this.aboutTab = new MetroFramework.Controls.MetroTabPage();
            this.metroTextBox3 = new MetroFramework.Controls.MetroTextBox();
            this.versionLabel = new MetroFramework.Controls.MetroLabel();
            this.metroLabel6 = new MetroFramework.Controls.MetroLabel();
            this.metroLabel3 = new MetroFramework.Controls.MetroLabel();
            this.startServerButton = new MetroFramework.Controls.MetroButton();
            this.timer1 = new System.Windows.Forms.Timer(this.components);
            this.metroTabControl1.SuspendLayout();
            this.serverTab.SuspendLayout();
            this.findingPanel.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)(this.dataGridView1)).BeginInit();
            this.runningPanel.SuspendLayout();
            this.debugTab.SuspendLayout();
            this.aboutTab.SuspendLayout();
            this.SuspendLayout();
            // 
            // metroButton1
            // 
            this.metroButton1.Location = new System.Drawing.Point(281, 63);
            this.metroButton1.Name = "metroButton1";
            this.metroButton1.Size = new System.Drawing.Size(75, 23);
            this.metroButton1.TabIndex = 6;
            this.metroButton1.Text = "Send";
            // 
            // metroButton2
            // 
            this.metroButton2.Location = new System.Drawing.Point(281, 104);
            this.metroButton2.Name = "metroButton2";
            this.metroButton2.Size = new System.Drawing.Size(75, 23);
            this.metroButton2.TabIndex = 6;
            this.metroButton2.Text = "Send";
            // 
            // metroProgressSpinner1
            // 
            this.metroProgressSpinner1.Location = new System.Drawing.Point(473, 32);
            this.metroProgressSpinner1.Maximum = 100;
            this.metroProgressSpinner1.Name = "metroProgressSpinner1";
            this.metroProgressSpinner1.Size = new System.Drawing.Size(38, 36);
            this.metroProgressSpinner1.TabIndex = 7;
            this.metroProgressSpinner1.Value = 30;
            // 
            // metroButton3
            // 
            this.metroButton3.Location = new System.Drawing.Point(281, 148);
            this.metroButton3.Name = "metroButton3";
            this.metroButton3.Size = new System.Drawing.Size(75, 23);
            this.metroButton3.TabIndex = 6;
            this.metroButton3.Text = "Capture DDS";
            this.metroButton3.Click += new System.EventHandler(this.metroButton3_Click);
            // 
            // metroTabControl1
            // 
            this.metroTabControl1.Controls.Add(this.serverTab);
            this.metroTabControl1.Controls.Add(this.debugTab);
            this.metroTabControl1.Controls.Add(this.aboutTab);
            this.metroTabControl1.Location = new System.Drawing.Point(23, 63);
            this.metroTabControl1.Name = "metroTabControl1";
            this.metroTabControl1.SelectedIndex = 1;
            this.metroTabControl1.Size = new System.Drawing.Size(664, 325);
            this.metroTabControl1.TabIndex = 8;
            // 
            // serverTab
            // 
            this.serverTab.Controls.Add(this.findingPanel);
            this.serverTab.Controls.Add(this.runningPanel);
            this.serverTab.HorizontalScrollbarBarColor = true;
            this.serverTab.Location = new System.Drawing.Point(4, 38);
            this.serverTab.Name = "serverTab";
            this.serverTab.Size = new System.Drawing.Size(656, 283);
            this.serverTab.TabIndex = 3;
            this.serverTab.Text = "Server";
            this.serverTab.VerticalScrollbarBarColor = true;
            // 
            // findingPanel
            // 
            this.findingPanel.Controls.Add(this.dataGridView1);
            this.findingPanel.Controls.Add(this.metroProgressSpinner2);
            this.findingPanel.Controls.Add(this.metroLabel4);
            this.findingPanel.HorizontalScrollbarBarColor = true;
            this.findingPanel.HorizontalScrollbarHighlightOnWheel = false;
            this.findingPanel.HorizontalScrollbarSize = 10;
            this.findingPanel.Location = new System.Drawing.Point(3, 3);
            this.findingPanel.Name = "findingPanel";
            this.findingPanel.Size = new System.Drawing.Size(657, 295);
            this.findingPanel.TabIndex = 11;
            this.findingPanel.VerticalScrollbarBarColor = true;
            this.findingPanel.VerticalScrollbarHighlightOnWheel = false;
            this.findingPanel.VerticalScrollbarSize = 10;
            // 
            // dataGridView1
            // 
            this.dataGridView1.AllowUserToAddRows = false;
            this.dataGridView1.AllowUserToDeleteRows = false;
            this.dataGridView1.AllowUserToResizeRows = false;
            this.dataGridView1.BackgroundColor = System.Drawing.Color.White;
            this.dataGridView1.CellBorderStyle = System.Windows.Forms.DataGridViewCellBorderStyle.None;
            this.dataGridView1.ColumnHeadersHeightSizeMode = System.Windows.Forms.DataGridViewColumnHeadersHeightSizeMode.AutoSize;
            this.dataGridView1.ColumnHeadersVisible = false;
            this.dataGridView1.Columns.AddRange(new System.Windows.Forms.DataGridViewColumn[] {
            this.NameColumn,
            this.AddressColumn,
            this.Button});
            this.dataGridView1.Location = new System.Drawing.Point(29, 28);
            this.dataGridView1.Name = "dataGridView1";
            this.dataGridView1.ReadOnly = true;
            this.dataGridView1.RowHeadersVisible = false;
            this.dataGridView1.RowTemplate.Height = 21;
            this.dataGridView1.SelectionMode = System.Windows.Forms.DataGridViewSelectionMode.FullRowSelect;
            this.dataGridView1.Size = new System.Drawing.Size(313, 233);
            this.dataGridView1.TabIndex = 2;
            this.dataGridView1.CellContentClick += new System.Windows.Forms.DataGridViewCellEventHandler(this.dataGridView1_CellContentClick);
            // 
            // NameColumn
            // 
            this.NameColumn.HeaderText = "Name";
            this.NameColumn.Name = "NameColumn";
            this.NameColumn.ReadOnly = true;
            // 
            // AddressColumn
            // 
            this.AddressColumn.HeaderText = "IPAddress";
            this.AddressColumn.Name = "AddressColumn";
            this.AddressColumn.ReadOnly = true;
            // 
            // Button
            // 
            this.Button.HeaderText = "Button";
            this.Button.Name = "Button";
            this.Button.ReadOnly = true;
            this.Button.Text = "Connect";
            // 
            // metroProgressSpinner2
            // 
            this.metroProgressSpinner2.Location = new System.Drawing.Point(383, 129);
            this.metroProgressSpinner2.Maximum = 100;
            this.metroProgressSpinner2.Name = "metroProgressSpinner2";
            this.metroProgressSpinner2.Size = new System.Drawing.Size(38, 36);
            this.metroProgressSpinner2.TabIndex = 7;
            this.metroProgressSpinner2.Value = 70;
            // 
            // metroLabel4
            // 
            this.metroLabel4.BackColor = System.Drawing.Color.White;
            this.metroLabel4.CustomBackground = true;
            this.metroLabel4.CustomForeColor = true;
            this.metroLabel4.FontWeight = MetroFramework.MetroLabelWeight.Regular;
            this.metroLabel4.Location = new System.Drawing.Point(442, 126);
            this.metroLabel4.Name = "metroLabel4";
            this.metroLabel4.Size = new System.Drawing.Size(115, 39);
            this.metroLabel4.TabIndex = 9;
            this.metroLabel4.Text = "Finding Client...";
            this.metroLabel4.TextAlign = System.Drawing.ContentAlignment.MiddleCenter;
            // 
            // runningPanel
            // 
            this.runningPanel.Controls.Add(this.runningLabel);
            this.runningPanel.HorizontalScrollbarBarColor = true;
            this.runningPanel.HorizontalScrollbarHighlightOnWheel = false;
            this.runningPanel.HorizontalScrollbarSize = 10;
            this.runningPanel.Location = new System.Drawing.Point(3, 3);
            this.runningPanel.Name = "runningPanel";
            this.runningPanel.Size = new System.Drawing.Size(657, 284);
            this.runningPanel.TabIndex = 10;
            this.runningPanel.VerticalScrollbarBarColor = true;
            this.runningPanel.VerticalScrollbarHighlightOnWheel = false;
            this.runningPanel.VerticalScrollbarSize = 10;
            // 
            // runningLabel
            // 
            this.runningLabel.BackColor = System.Drawing.Color.White;
            this.runningLabel.CustomBackground = true;
            this.runningLabel.CustomForeColor = true;
            this.runningLabel.FontWeight = MetroFramework.MetroLabelWeight.Regular;
            this.runningLabel.Location = new System.Drawing.Point(287, 129);
            this.runningLabel.Name = "runningLabel";
            this.runningLabel.Size = new System.Drawing.Size(115, 39);
            this.runningLabel.TabIndex = 9;
            this.runningLabel.Text = "Running!";
            this.runningLabel.TextAlign = System.Drawing.ContentAlignment.MiddleCenter;
            // 
            // debugTab
            // 
            this.debugTab.Controls.Add(this.metroLabel5);
            this.debugTab.Controls.Add(this.logText);
            this.debugTab.Controls.Add(this.metroCheckBox3);
            this.debugTab.Controls.Add(this.metroCheckBox2);
            this.debugTab.Controls.Add(this.metroCheckBox1);
            this.debugTab.Controls.Add(this.metroTextBox2);
            this.debugTab.Controls.Add(this.metroTextBox1);
            this.debugTab.Controls.Add(this.metroLabel2);
            this.debugTab.Controls.Add(this.metroLabel1);
            this.debugTab.Controls.Add(this.metroButton5);
            this.debugTab.Controls.Add(this.metroButton4);
            this.debugTab.Controls.Add(this.metroButton3);
            this.debugTab.Controls.Add(this.metroButton2);
            this.debugTab.Controls.Add(this.metroButton1);
            this.debugTab.HorizontalScrollbarBarColor = true;
            this.debugTab.Location = new System.Drawing.Point(4, 38);
            this.debugTab.Name = "debugTab";
            this.debugTab.Size = new System.Drawing.Size(656, 283);
            this.debugTab.TabIndex = 2;
            this.debugTab.Text = "Debug";
            this.debugTab.VerticalScrollbarBarColor = true;
            // 
            // metroLabel5
            // 
            this.metroLabel5.AutoSize = true;
            this.metroLabel5.CustomForeColor = true;
            this.metroLabel5.ForeColor = System.Drawing.Color.Red;
            this.metroLabel5.Location = new System.Drawing.Point(88, 20);
            this.metroLabel5.Name = "metroLabel5";
            this.metroLabel5.Size = new System.Drawing.Size(148, 19);
            this.metroLabel5.TabIndex = 11;
            this.metroLabel5.Text = "Do not touch this panel!";
            // 
            // logText
            // 
            this.logText.Location = new System.Drawing.Point(384, 11);
            this.logText.Multiline = true;
            this.logText.Name = "logText";
            this.logText.ReadOnly = true;
            this.logText.Size = new System.Drawing.Size(246, 269);
            this.logText.TabIndex = 10;
            // 
            // metroCheckBox3
            // 
            this.metroCheckBox3.AutoSize = true;
            this.metroCheckBox3.Location = new System.Drawing.Point(281, 258);
            this.metroCheckBox3.Name = "metroCheckBox3";
            this.metroCheckBox3.Size = new System.Drawing.Size(56, 15);
            this.metroCheckBox3.TabIndex = 9;
            this.metroCheckBox3.Text = "Mutex";
            this.metroCheckBox3.UseVisualStyleBackColor = true;
            this.metroCheckBox3.CheckedChanged += new System.EventHandler(this.metroCheckBox3_CheckedChanged);
            // 
            // metroCheckBox2
            // 
            this.metroCheckBox2.AutoSize = true;
            this.metroCheckBox2.Location = new System.Drawing.Point(156, 258);
            this.metroCheckBox2.Name = "metroCheckBox2";
            this.metroCheckBox2.Size = new System.Drawing.Size(68, 15);
            this.metroCheckBox2.TabIndex = 9;
            this.metroCheckBox2.Text = "Suspend";
            this.metroCheckBox2.UseVisualStyleBackColor = true;
            this.metroCheckBox2.CheckedChanged += new System.EventHandler(this.metroCheckBox2_CheckedChanged);
            // 
            // metroCheckBox1
            // 
            this.metroCheckBox1.AutoSize = true;
            this.metroCheckBox1.Location = new System.Drawing.Point(156, 226);
            this.metroCheckBox1.Name = "metroCheckBox1";
            this.metroCheckBox1.Size = new System.Drawing.Size(119, 15);
            this.metroCheckBox1.TabIndex = 9;
            this.metroCheckBox1.Text = "DebugFrameIndex";
            this.metroCheckBox1.UseVisualStyleBackColor = true;
            // 
            // metroTextBox2
            // 
            this.metroTextBox2.Location = new System.Drawing.Point(159, 104);
            this.metroTextBox2.Name = "metroTextBox2";
            this.metroTextBox2.Size = new System.Drawing.Size(116, 23);
            this.metroTextBox2.TabIndex = 8;
            this.metroTextBox2.Text = "0";
            // 
            // metroTextBox1
            // 
            this.metroTextBox1.Location = new System.Drawing.Point(159, 62);
            this.metroTextBox1.Name = "metroTextBox1";
            this.metroTextBox1.Size = new System.Drawing.Size(116, 23);
            this.metroTextBox1.TabIndex = 8;
            this.metroTextBox1.Text = "0";
            // 
            // metroLabel2
            // 
            this.metroLabel2.AutoSize = true;
            this.metroLabel2.Location = new System.Drawing.Point(1, 108);
            this.metroLabel2.Name = "metroLabel2";
            this.metroLabel2.Size = new System.Drawing.Size(139, 19);
            this.metroLabel2.TabIndex = 7;
            this.metroLabel2.Text = "EnableDriverTestMode";
            // 
            // metroLabel1
            // 
            this.metroLabel1.AutoSize = true;
            this.metroLabel1.Location = new System.Drawing.Point(1, 67);
            this.metroLabel1.Name = "metroLabel1";
            this.metroLabel1.Size = new System.Drawing.Size(104, 19);
            this.metroLabel1.TabIndex = 7;
            this.metroLabel1.Text = "EnableTestMode";
            // 
            // metroButton5
            // 
            this.metroButton5.Location = new System.Drawing.Point(281, 226);
            this.metroButton5.Name = "metroButton5";
            this.metroButton5.Size = new System.Drawing.Size(75, 23);
            this.metroButton5.TabIndex = 6;
            this.metroButton5.Text = "Send";
            this.metroButton5.Click += new System.EventHandler(this.metroButton5_Click);
            // 
            // metroButton4
            // 
            this.metroButton4.Location = new System.Drawing.Point(281, 188);
            this.metroButton4.Name = "metroButton4";
            this.metroButton4.Size = new System.Drawing.Size(75, 23);
            this.metroButton4.TabIndex = 6;
            this.metroButton4.Text = "GetConfig";
            this.metroButton4.Click += new System.EventHandler(this.metroButton4_Click);
            // 
            // aboutTab
            // 
            this.aboutTab.Controls.Add(this.metroTextBox3);
            this.aboutTab.Controls.Add(this.versionLabel);
            this.aboutTab.Controls.Add(this.metroLabel6);
            this.aboutTab.HorizontalScrollbarBarColor = true;
            this.aboutTab.Location = new System.Drawing.Point(4, 38);
            this.aboutTab.Name = "aboutTab";
            this.aboutTab.Size = new System.Drawing.Size(656, 283);
            this.aboutTab.TabIndex = 4;
            this.aboutTab.Text = "About";
            this.aboutTab.VerticalScrollbarBarColor = true;
            // 
            // metroTextBox3
            // 
            this.metroTextBox3.Location = new System.Drawing.Point(297, 15);
            this.metroTextBox3.Multiline = true;
            this.metroTextBox3.Name = "metroTextBox3";
            this.metroTextBox3.ReadOnly = true;
            this.metroTextBox3.Size = new System.Drawing.Size(324, 253);
            this.metroTextBox3.TabIndex = 3;
            // 
            // versionLabel
            // 
            this.versionLabel.AutoSize = true;
            this.versionLabel.FontSize = MetroFramework.MetroLabelSize.Tall;
            this.versionLabel.Location = new System.Drawing.Point(89, 59);
            this.versionLabel.Name = "versionLabel";
            this.versionLabel.Size = new System.Drawing.Size(39, 25);
            this.versionLabel.TabIndex = 2;
            this.versionLabel.Text = "v1.0";
            // 
            // metroLabel6
            // 
            this.metroLabel6.AutoSize = true;
            this.metroLabel6.FontSize = MetroFramework.MetroLabelSize.Tall;
            this.metroLabel6.Location = new System.Drawing.Point(33, 59);
            this.metroLabel6.Name = "metroLabel6";
            this.metroLabel6.Size = new System.Drawing.Size(50, 25);
            this.metroLabel6.TabIndex = 2;
            this.metroLabel6.Text = "ALVR";
            // 
            // metroLabel3
            // 
            this.metroLabel3.CustomBackground = true;
            this.metroLabel3.CustomForeColor = true;
            this.metroLabel3.FontWeight = MetroFramework.MetroLabelWeight.Regular;
            this.metroLabel3.Location = new System.Drawing.Point(517, 32);
            this.metroLabel3.Name = "metroLabel3";
            this.metroLabel3.Size = new System.Drawing.Size(115, 39);
            this.metroLabel3.TabIndex = 9;
            this.metroLabel3.Text = "metroLabel3";
            this.metroLabel3.TextAlign = System.Drawing.ContentAlignment.MiddleCenter;
            // 
            // startServerButton
            // 
            this.startServerButton.Location = new System.Drawing.Point(376, 32);
            this.startServerButton.Name = "startServerButton";
            this.startServerButton.Size = new System.Drawing.Size(75, 36);
            this.startServerButton.TabIndex = 10;
            this.startServerButton.Text = "Start server";
            this.startServerButton.Click += new System.EventHandler(this.metroButton6_Click);
            // 
            // timer1
            // 
            this.timer1.Enabled = true;
            this.timer1.Interval = 1000;
            this.timer1.Tick += new System.EventHandler(this.timer1_Tick);
            // 
            // Launcher
            // 
            this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
            this.ClientSize = new System.Drawing.Size(710, 411);
            this.Controls.Add(this.startServerButton);
            this.Controls.Add(this.metroProgressSpinner1);
            this.Controls.Add(this.metroLabel3);
            this.Controls.Add(this.metroTabControl1);
            this.Name = "Launcher";
            this.Text = "ALVR";
            this.Load += new System.EventHandler(this.Launcher_Load);
            this.metroTabControl1.ResumeLayout(false);
            this.serverTab.ResumeLayout(false);
            this.findingPanel.ResumeLayout(false);
            ((System.ComponentModel.ISupportInitialize)(this.dataGridView1)).EndInit();
            this.runningPanel.ResumeLayout(false);
            this.debugTab.ResumeLayout(false);
            this.debugTab.PerformLayout();
            this.aboutTab.ResumeLayout(false);
            this.aboutTab.PerformLayout();
            this.ResumeLayout(false);

        }

        #endregion

        private MetroFramework.Controls.MetroButton metroButton1;
        private MetroFramework.Controls.MetroButton metroButton2;
        private MetroFramework.Controls.MetroProgressSpinner metroProgressSpinner1;
        private MetroFramework.Controls.MetroButton metroButton3;
        private MetroFramework.Controls.MetroTabControl metroTabControl1;
        private MetroFramework.Controls.MetroTabPage debugTab;
        private MetroFramework.Controls.MetroTextBox metroTextBox2;
        private MetroFramework.Controls.MetroTextBox metroTextBox1;
        private MetroFramework.Controls.MetroLabel metroLabel2;
        private MetroFramework.Controls.MetroLabel metroLabel1;
        private MetroFramework.Controls.MetroLabel metroLabel3;
        private MetroFramework.Controls.MetroButton metroButton4;
        private MetroFramework.Controls.MetroCheckBox metroCheckBox1;
        private MetroFramework.Controls.MetroButton metroButton5;
        private MetroFramework.Controls.MetroCheckBox metroCheckBox2;
        private MetroFramework.Controls.MetroCheckBox metroCheckBox3;
        private MetroFramework.Controls.MetroTextBox logText;
        private MetroFramework.Controls.MetroButton startServerButton;
        private MetroFramework.Controls.MetroTabPage serverTab;
        private System.Windows.Forms.DataGridView dataGridView1;
        private System.Windows.Forms.Timer timer1;
        private MetroFramework.Controls.MetroLabel metroLabel4;
        private MetroFramework.Controls.MetroProgressSpinner metroProgressSpinner2;
        private System.Windows.Forms.DataGridViewTextBoxColumn NameColumn;
        private System.Windows.Forms.DataGridViewTextBoxColumn AddressColumn;
        private System.Windows.Forms.DataGridViewButtonColumn Button;
        private MetroFramework.Controls.MetroLabel runningLabel;
        private MetroFramework.Controls.MetroPanel findingPanel;
        private MetroFramework.Controls.MetroPanel runningPanel;
        private MetroFramework.Controls.MetroLabel metroLabel5;
        private MetroFramework.Controls.MetroTabPage aboutTab;
        private MetroFramework.Controls.MetroTextBox metroTextBox3;
        private MetroFramework.Controls.MetroLabel versionLabel;
        private MetroFramework.Controls.MetroLabel metroLabel6;
    }
}

