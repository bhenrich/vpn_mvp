using System.Diagnostics;
using System.Runtime.Versioning;
using System.Threading.Tasks;

namespace App.Services.Windows;

public static class AdminProcess
{
	[SupportedOSPlatform("windows")]
	public static async Task<int> RunElevatedAsync(string fileName, string arguments)
	{
		var psi = new ProcessStartInfo
		{
			FileName = fileName,
			Arguments = arguments,
			UseShellExecute = true,
			Verb = "runas",
			CreateNoWindow = true,
		};
		using var proc = Process.Start(psi)!;
		await proc.WaitForExitAsync().ConfigureAwait(false);
		return proc.ExitCode;
	}
}


