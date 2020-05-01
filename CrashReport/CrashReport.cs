using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace CrashReport
{
    public partial class CrashReport : Form
    {
        public CrashReport()
        {
            InitializeComponent();

            textBox1.Text = Environment.GetCommandLineArgs()[1];
            textBox1.Select(0, 0);

            CenterToScreen();
        }
    }
}
