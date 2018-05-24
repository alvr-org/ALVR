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
            this.metroTabPage4 = new MetroFramework.Controls.MetroTabPage();
            this.dataGridView1 = new System.Windows.Forms.DataGridView();
            this.metroTabPage1 = new MetroFramework.Controls.MetroTabPage();
            this.metroTabPage3 = new MetroFramework.Controls.MetroTabPage();
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
            this.metroTabPage2 = new MetroFramework.Controls.MetroTabPage();
            this.metroLabel3 = new MetroFramework.Controls.MetroLabel();
            this.startServerButton = new MetroFramework.Controls.MetroButton();
            this.timer1 = new System.Windows.Forms.Timer(this.components);
            this.metroProgressSpinner2 = new MetroFramework.Controls.MetroProgressSpinner();
            this.metroLabel4 = new MetroFramework.Controls.MetroLabel();
            this.NameColumn = new System.Windows.Forms.DataGridViewTextBoxColumn();
            this.AddressColumn = new System.Windows.Forms.DataGridViewTextBoxColumn();
            this.Button = new System.Windows.Forms.DataGridViewButtonColumn();
            this.runningLabel = new MetroFramework.Controls.MetroLabel();
            this.runningPanel = new MetroFramework.Controls.MetroPanel();
            this.findingPanel = new MetroFramework.Controls.MetroPanel();
            this.metroTabControl1.SuspendLayout();
            this.metroTabPage4.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)(this.dataGridView1)).BeginInit();
            this.metroTabPage3.SuspendLayout();
            this.runningPanel.SuspendLayout();
            this.findingPanel.SuspendLayout();
            this.SuspendLayout();
            // 
            // metroButton1
            // 
            this.metroButton1.Location = new System.Drawing.Point(283, 12);
            this.metroButton1.Name = "metroButton1";
            this.metroButton1.Size = new System.Drawing.Size(75, 23);
            this.metroButton1.TabIndex = 6;
            this.metroButton1.Text = "Send";
            // 
            // metroButton2
            // 
            this.metroButton2.Location = new System.Drawing.Point(283, 53);
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
            this.metroButton3.Location = new System.Drawing.Point(283, 97);
            this.metroButton3.Name = "metroButton3";
            this.metroButton3.Size = new System.Drawing.Size(75, 23);
            this.metroButton3.TabIndex = 6;
            this.metroButton3.Text = "Capture DDS";
            // 
            // metroTabControl1
            // 
            this.metroTabControl1.Controls.Add(this.metroTabPage4);
            this.metroTabControl1.Controls.Add(this.metroTabPage3);
            this.metroTabControl1.Controls.Add(this.metroTabPage1);
            this.metroTabControl1.Controls.Add(this.metroTabPage2);
            this.metroTabControl1.Location = new System.Drawing.Point(23, 63);
            this.metroTabControl1.Name = "metroTabControl1";
            this.metroTabControl1.SelectedIndex = 1;
            this.metroTabControl1.Size = new System.Drawing.Size(664, 325);
            this.metroTabControl1.TabIndex = 8;
            // 
            // metroTabPage4
            // 
            this.metroTabPage4.Controls.Add(this.findingPanel);
            this.metroTabPage4.Controls.Add(this.runningPanel);
            this.metroTabPage4.HorizontalScrollbarBarColor = true;
            this.metroTabPage4.Location = new System.Drawing.Point(4, 38);
            this.metroTabPage4.Name = "metroTabPage4";
            this.metroTabPage4.Size = new System.Drawing.Size(656, 283);
            this.metroTabPage4.TabIndex = 3;
            this.metroTabPage4.Text = "Server";
            this.metroTabPage4.VerticalScrollbarBarColor = true;
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
            // metroTabPage1
            // 
            this.metroTabPage1.HorizontalScrollbarBarColor = true;
            this.metroTabPage1.Location = new System.Drawing.Point(4, 38);
            this.metroTabPage1.Name = "metroTabPage1";
            this.metroTabPage1.Size = new System.Drawing.Size(656, 283);
            this.metroTabPage1.TabIndex = 0;
            this.metroTabPage1.Text = "Video";
            this.metroTabPage1.VerticalScrollbarBarColor = true;
            // 
            // metroTabPage3
            // 
            this.metroTabPage3.Controls.Add(this.logText);
            this.metroTabPage3.Controls.Add(this.metroCheckBox3);
            this.metroTabPage3.Controls.Add(this.metroCheckBox2);
            this.metroTabPage3.Controls.Add(this.metroCheckBox1);
            this.metroTabPage3.Controls.Add(this.metroTextBox2);
            this.metroTabPage3.Controls.Add(this.metroTextBox1);
            this.metroTabPage3.Controls.Add(this.metroLabel2);
            this.metroTabPage3.Controls.Add(this.metroLabel1);
            this.metroTabPage3.Controls.Add(this.metroButton5);
            this.metroTabPage3.Controls.Add(this.metroButton4);
            this.metroTabPage3.Controls.Add(this.metroButton3);
            this.metroTabPage3.Controls.Add(this.metroButton2);
            this.metroTabPage3.Controls.Add(this.metroButton1);
            this.metroTabPage3.HorizontalScrollbarBarColor = true;
            this.metroTabPage3.Location = new System.Drawing.Point(4, 38);
            this.metroTabPage3.Name = "metroTabPage3";
            this.metroTabPage3.Size = new System.Drawing.Size(656, 283);
            this.metroTabPage3.TabIndex = 2;
            this.metroTabPage3.Text = "Debug";
            this.metroTabPage3.VerticalScrollbarBarColor = true;
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
            this.metroCheckBox3.Location = new System.Drawing.Point(283, 207);
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
            this.metroCheckBox2.Location = new System.Drawing.Point(158, 207);
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
            this.metroCheckBox1.Location = new System.Drawing.Point(158, 175);
            this.metroCheckBox1.Name = "metroCheckBox1";
            this.metroCheckBox1.Size = new System.Drawing.Size(119, 15);
            this.metroCheckBox1.TabIndex = 9;
            this.metroCheckBox1.Text = "DebugFrameIndex";
            this.metroCheckBox1.UseVisualStyleBackColor = true;
            // 
            // metroTextBox2
            // 
            this.metroTextBox2.Location = new System.Drawing.Point(161, 53);
            this.metroTextBox2.Name = "metroTextBox2";
            this.metroTextBox2.Size = new System.Drawing.Size(116, 23);
            this.metroTextBox2.TabIndex = 8;
            this.metroTextBox2.Text = "0";
            // 
            // metroTextBox1
            // 
            this.metroTextBox1.Location = new System.Drawing.Point(161, 11);
            this.metroTextBox1.Name = "metroTextBox1";
            this.metroTextBox1.Size = new System.Drawing.Size(116, 23);
            this.metroTextBox1.TabIndex = 8;
            this.metroTextBox1.Text = "0";
            // 
            // metroLabel2
            // 
            this.metroLabel2.AutoSize = true;
            this.metroLabel2.Location = new System.Drawing.Point(3, 57);
            this.metroLabel2.Name = "metroLabel2";
            this.metroLabel2.Size = new System.Drawing.Size(139, 19);
            this.metroLabel2.TabIndex = 7;
            this.metroLabel2.Text = "EnableDriverTestMode";
            // 
            // metroLabel1
            // 
            this.metroLabel1.AutoSize = true;
            this.metroLabel1.Location = new System.Drawing.Point(3, 16);
            this.metroLabel1.Name = "metroLabel1";
            this.metroLabel1.Size = new System.Drawing.Size(104, 19);
            this.metroLabel1.TabIndex = 7;
            this.metroLabel1.Text = "EnableTestMode";
            // 
            // metroButton5
            // 
            this.metroButton5.Location = new System.Drawing.Point(283, 175);
            this.metroButton5.Name = "metroButton5";
            this.metroButton5.Size = new System.Drawing.Size(75, 23);
            this.metroButton5.TabIndex = 6;
            this.metroButton5.Text = "Send";
            this.metroButton5.Click += new System.EventHandler(this.metroButton5_Click);
            // 
            // metroButton4
            // 
            this.metroButton4.Location = new System.Drawing.Point(283, 137);
            this.metroButton4.Name = "metroButton4";
            this.metroButton4.Size = new System.Drawing.Size(75, 23);
            this.metroButton4.TabIndex = 6;
            this.metroButton4.Text = "GetConfig";
            this.metroButton4.Click += new System.EventHandler(this.metroButton4_Click);
            // 
            // metroTabPage2
            // 
            this.metroTabPage2.HorizontalScrollbarBarColor = true;
            this.metroTabPage2.Location = new System.Drawing.Point(4, 38);
            this.metroTabPage2.Name = "metroTabPage2";
            this.metroTabPage2.Size = new System.Drawing.Size(656, 283);
            this.metroTabPage2.TabIndex = 1;
            this.metroTabPage2.Text = "Network";
            this.metroTabPage2.VerticalScrollbarBarColor = true;
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
            this.metroTabPage4.ResumeLayout(false);
            ((System.ComponentModel.ISupportInitialize)(this.dataGridView1)).EndInit();
            this.metroTabPage3.ResumeLayout(false);
            this.metroTabPage3.PerformLayout();
            this.runningPanel.ResumeLayout(false);
            this.findingPanel.ResumeLayout(false);
            this.ResumeLayout(false);

        }

        #endregion

        private MetroFramework.Controls.MetroButton metroButton1;
        private MetroFramework.Controls.MetroButton metroButton2;
        private MetroFramework.Controls.MetroProgressSpinner metroProgressSpinner1;
        private MetroFramework.Controls.MetroButton metroButton3;
        private MetroFramework.Controls.MetroTabControl metroTabControl1;
        private MetroFramework.Controls.MetroTabPage metroTabPage1;
        private MetroFramework.Controls.MetroTabPage metroTabPage2;
        private MetroFramework.Controls.MetroTabPage metroTabPage3;
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
        private MetroFramework.Controls.MetroTabPage metroTabPage4;
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
    }
}

