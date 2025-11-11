using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

namespace App.Services;

public sealed class TokenStorage
{
	private readonly string _path;
	private readonly JsonSerializerOptions _json = new(JsonSerializerDefaults.Web);

	public TokenStorage()
	{
		var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
		var dir = Path.Combine(appData, "VpnMvp");
		Directory.CreateDirectory(dir);
		_path = Path.Combine(dir, "tokens.json");
	}

	public sealed record Tokens(string AccessToken, string RefreshToken, string Email);

	public async Task SaveAsync(string accessToken, string refreshToken, string email)
	{
		var tokens = new Tokens(accessToken, refreshToken, email);
		var data = JsonSerializer.SerializeToUtf8Bytes(tokens, _json);
		var protectedBytes = Protect(data);
		await File.WriteAllBytesAsync(_path, protectedBytes).ConfigureAwait(false);
	}

	public async Task<Tokens> LoadAsync()
	{
		var bytes = await File.ReadAllBytesAsync(_path).ConfigureAwait(false);
		var data = Unprotect(bytes);
		var tokens = JsonSerializer.Deserialize<Tokens>(data, _json);
		return tokens!;
	}

	private static byte[] Protect(byte[] data)
	{
		try
		{
			return ProtectedData.Protect(data, null, DataProtectionScope.CurrentUser);
		}
		catch
		{
			return data;
		}
	}

	private static byte[] Unprotect(byte[] data)
	{
		try
		{
			return ProtectedData.Unprotect(data, null, DataProtectionScope.CurrentUser);
		}
		catch
		{
			return data;
		}
	}
}


