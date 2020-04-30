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

            textBox1.Text = UserFriendlyError(Environment.GetCommandLineArgs()[1]);
            textBox1.Select(0, 0);

            CenterToScreen();
        }

        private string UserFriendlyError(string error)
        {
            if (error == null)
            {
                return null;
            }

            if (error.Contains("Failed to initialize CEncoder. All VideoEncoder are not available. VCE: AMF Error 4. m_amfContext->InitDX11(m_d3dRender->GetDevice()), NVENC: Failed to load nvcuda.dll. Please check if NVIDIA graphic driver is installed."))
            {
                return error + " *** This error has been reported to occur when you are running an unsupported GPU, for example an integrated GPU of a CPU, as a main GPU of your system. Please ensure the cable of your monitor is connected to a supported dedicated GPU directly. For more information see github.com/JackD83/ALVR/issues/130";
            }

            return error;
        }
    }
}
