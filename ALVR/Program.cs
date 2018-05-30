using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace ALVR
{
    static class Program
    {
        private static readonly string mutexName = "Global\\ALVR_MUTEX_10C22CCD-9962-4F2F-9E9E-4251A32848C3";
        [STAThread]
        static void Main()
        {
            using (Mutex mutex = new Mutex(false, mutexName))
            {
                if (!mutex.WaitOne(0, false))
                {
                    MessageBox.Show("Already running!");
                    return;
                }

                Application.EnableVisualStyles();
                Application.SetCompatibleTextRenderingDefault(false);
                Application.Run(new Launcher());
            }
        }
    }
}
