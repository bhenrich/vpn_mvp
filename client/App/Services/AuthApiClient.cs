using System.Net.Http;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

namespace App.Services;

public sealed class AuthApiClient
{
	private readonly HttpClient _http;
	private readonly JsonSerializerOptions _json = new(JsonSerializerDefaults.Web);

	public AuthApiClient()
	{
		var baseUrl = Environment.GetEnvironmentVariable("AUTH_BASE_URL") ?? "http://localhost:8080";
		_http = new HttpClient { BaseAddress = new System.Uri(baseUrl) };
	}

	public sealed record LoginResponse(string AccessToken, string RefreshToken);

	public async Task<LoginResponse> LoginAsync(string email, string password)
	{
		var payload = JsonSerializer.Serialize(new { email, password }, _json);
		using var content = new StringContent(payload, Encoding.UTF8, "application/json");
		using var resp = await _http.PostAsync("/api/v1/auth/login", content).ConfigureAwait(false);
		resp.EnsureSuccessStatusCode();
		var result = await resp.Content.ReadFromJsonAsync<LoginResponse>(_json).ConfigureAwait(false);
		return result!;
	}

	public async Task<string> FetchProfileAsync(string accessToken)
	{
		using var req = new HttpRequestMessage(HttpMethod.Get, "/api/v1/vpn/profile");
		req.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", accessToken);
		using var resp = await _http.SendAsync(req).ConfigureAwait(false);
		resp.EnsureSuccessStatusCode();
		return await resp.Content.ReadAsStringAsync().ConfigureAwait(false);
	}
}


