param(
    [string]$lnkPath,
    [string]$targetPath,
    [string]$aumid
)

# 用 C# 通过 COM 创建 .lnk 并写入 System.AppUserModel.ID 属性，
# 这是让普通（非安装）桌面程序正常显示 Toast 的必要身份。
Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Text;
using System.Runtime.InteropServices;

public class LnkAumid {
    [ComImport, Guid("00021401-0000-0000-C000-000000000046"), ClassInterface(ClassInterfaceType.None)]
    private class ShellLink {}

    [ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellLinkW {
        void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, IntPtr pfd, int fFlags);
        void GetIDList(out IntPtr ppidl);
        void SetIDList(IntPtr pidl);
        void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);
        void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
        void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);
        void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
        void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);
        void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
        void GetHotkey(out short pwHotkey);
        void SetHotkey(short wHotkey);
        void GetShowCmd(out int piShowCmd);
        void SetShowCmd(int iShowCmd);
        void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);
        void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
        void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, int dwReserved);
        void Resolve(IntPtr hwnd, uint fFlags);
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
    }

    [ComImport, Guid("0000010B-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPersistFile {
        void GetClassID(out Guid pClassID);
        void IsDirty();
        void Load([MarshalAs(UnmanagedType.LPWStr)] string pszFileName, uint dwMode);
        void Save([MarshalAs(UnmanagedType.LPWStr)] string pszFileName, [MarshalAs(UnmanagedType.Bool)] bool fRemember);
        void SaveCompleted([MarshalAs(UnmanagedType.LPWStr)] string pszFileName);
        void GetCurFile(out string ppszFileName);
    }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore {
        void GetCount(out uint cProps);
        void GetAt(uint iProp, out PropKey pkey);
        void GetValue([In] ref PropKey key, out PropVariant pv);
        void SetValue([In] ref PropKey key, [In] ref PropVariant pv);
        void Commit();
    }

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    private struct PropKey { public Guid fmtid; public uint pid; }

    [StructLayout(LayoutKind.Sequential)]
    private struct PropVariant { public ushort vt; public ushort wReserved1; public ushort wReserved2; public ushort wReserved3; public IntPtr pVal; }

    [DllImport("propsys.dll")]
    private static extern int PSCreatePropertyStoreFromObject(IntPtr pUnk, uint cbExtra, [In] ref Guid riid, out IntPtr ppv);

    public static void Create(string lnkPath, string targetPath, string aumid) {
        Guid iidPropStore = new Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99");
        PropKey pkeyAumid = new PropKey { fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid = 5 };

        var link = (IShellLinkW)new ShellLink();
        link.SetPath(targetPath);
        link.SetWorkingDirectory(Path.GetDirectoryName(targetPath));
        link.SetDescription("ClipDedup");
        var pf = (IPersistFile)link;
        pf.Save(lnkPath, true);

        IntPtr pps;
        int hr = PSCreatePropertyStoreFromObject(Marshal.GetIUnknownForObject(link), 0, ref iidPropStore, out pps);
        if (hr == 0 && pps != IntPtr.Zero) {
            var store = (IPropertyStore)Marshal.GetObjectForIUnknown(pps);
            var pv = new PropVariant { vt = 31, pVal = Marshal.StringToCoTaskMemUni(aumid) };
            store.SetValue(ref pkeyAumid, ref pv);
            store.Commit();
            Marshal.FreeCoTaskMem(pv.pVal);
            Marshal.Release(pps);
            // 属性写入 .lnk 附加数据后需重新保存以落盘
            pf.Save(lnkPath, true);
        } else {
            Console.Error.WriteLine("PSCreatePropertyStoreFromObject hr=" + hr);
        }
        Marshal.ReleaseComObject(link);
    }
}
'@

[LnkAumid]::Create($lnkPath, $targetPath, $aumid)
