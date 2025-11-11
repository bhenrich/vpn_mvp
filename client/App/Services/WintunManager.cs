using System;
using System.IO;
using System.Linq;
using System.Runtime.Versioning;
using System.Threading.Tasks;
using App.Services.Windows;
using Microsoft.Win32;

namespace App.Services;

public sealed class WintunManager
{
	[SupportedOSPlatform("windows")]
	public async Task<bool> EnsureInstalledAsync()
	{
		if (IsInstalled())
		{
			return true;
		}
		var inf = FindInf();
		if (inf is not null)
		{
			// Install via INF (requires elevation)
			try
			{
				var code = await AdminProcess.RunElevatedAsync("pnputil.exe", $"/add-driver \"{inf}\" /install").ConfigureAwait(false);
				if (code == 0)
				{
					await Task.Delay(1000).ConfigureAwait(false);
					return IsInstalled();
				}
			}
			catch
			{
				// User may have cancelled UAC or pnputil not found
			}
		}

		// Fallback: attempt install using wintun.dll (requires elevation)
		if (OperatingSystem.IsWindows() && WintunNative.TryInstallDriverFromDll())
		{
			await Task.Delay(1000).ConfigureAwait(false);
			return IsInstalled();
		}

		return false;
	}

	[SupportedOSPlatform("windows")]
	private static bool IsInstalled()
	{
		try
		{
			using var key = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\Wintun");
			if (key is not null) return true;
		}
		catch
		{
			// ignore
		}

		// Fallback: check via DLL API if the driver is running/available
		return OperatingSystem.IsWindows() && WintunNative.IsDriverLoaded();
	}

	private static string? FindInf()
	{
		var baseDir = AppContext.BaseDirectory;
		var root = Path.Combine(baseDir, "Native", "wintun");
		if (!Directory.Exists(root)) return null;
		return Directory.EnumerateFiles(root, "*.inf", SearchOption.AllDirectories)
			.FirstOrDefault(f => string.Equals(Path.GetFileName(f), "wintun.inf", StringComparison.OrdinalIgnoreCase));
	}
}


