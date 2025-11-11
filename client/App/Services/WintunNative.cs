using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;

namespace App.Services;

internal static class WintunNative
{
	[DllImport("wintun", EntryPoint = "WintunCreateAdapter", CharSet = CharSet.Unicode, SetLastError = true)]
	private static extern IntPtr WintunCreateAdapter(string name, string tunnelType, IntPtr requestedGuid);

	[DllImport("wintun", EntryPoint = "WintunCloseAdapter", SetLastError = true)]
	private static extern void WintunCloseAdapter(IntPtr adapter);

	[DllImport("wintun", EntryPoint = "WintunGetRunningDriverVersion", SetLastError = true)]
	private static extern uint WintunGetRunningDriverVersion();

	private static bool EnsureDllLoaded()
	{
		// Try likely locations for wintun.dll
		var baseDir = AppContext.BaseDirectory;
		var programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
		var envOverride = Environment.GetEnvironmentVariable("WINTUN_DLL");

		string[] candidates = new[]
		{
			envOverride ?? string.Empty,
			Path.Combine(baseDir, "Native", "wintun", "wintun.dll"),
			Path.Combine(baseDir, "wintun.dll"),
			Path.Combine(baseDir, "runtimes", "win-x64", "native", "wintun.dll"),
			Path.Combine(programFiles, "WireGuard", "wintun.dll"),
		};

		foreach (var path in candidates)
		{
			try
			{
				if (!string.IsNullOrWhiteSpace(path) && File.Exists(path))
				{
					if (NativeLibrary.TryLoad(path, out _))
					{
						return true;
					}
				}
			}
			catch
			{
				// ignore and try next
			}
		}

		// As a last resort, let DllImport resolve from PATH
		return true;
	}

	[SupportedOSPlatform("windows")]
	public static bool IsDriverLoaded()
	{
		try
		{
			EnsureDllLoaded();
			return WintunGetRunningDriverVersion() != 0;
		}
		catch
		{
			return false;
		}
	}

	[SupportedOSPlatform("windows")]
	public static bool TryInstallDriverFromDll()
	{
		if (!EnsureDllLoaded())
		{
			return false;
		}

		// If already installed, we're done
		try
		{
			if (WintunGetRunningDriverVersion() != 0)
			{
				return true;
			}
		}
		catch
		{
			// If calling into the DLL failed, it's likely not present or couldn't be loaded
			return false;
		}

		// Try to create a temporary adapter to trigger driver install via the DLL.
		// This typically requires elevation; without it, CreateAdapter will fail.
		try
		{
			var adapter = WintunCreateAdapter("vpnmvp-temp", "Wintun", IntPtr.Zero);
			if (adapter != IntPtr.Zero)
			{
				WintunCloseAdapter(adapter);
				// After creating/closing adapter, driver should be present.
				return true;
			}
		}
		catch
		{
			// Creating the adapter failed (likely due to lack of elevation or missing DLL)
		}
		return false;
	}
}


