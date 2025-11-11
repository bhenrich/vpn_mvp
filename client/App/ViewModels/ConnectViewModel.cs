using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using System.Windows.Input;
using App.Services;
using System;

namespace App.ViewModels;

public class ConnectViewModel : INotifyPropertyChanged
{
	private readonly AuthApiClient _authApiClient;
	private readonly TokenStorage _tokenStorage;
	private readonly VpnSessionManager _vpn;
	private readonly WintunManager _wintun = new();
	private string _statusText = "Disconnected";
	private bool _isConnecting;
	private bool _isConnected;

	public event PropertyChangedEventHandler? PropertyChanged;

	public ConnectViewModel(AuthApiClient authApiClient, TokenStorage tokenStorage)
	{
		_authApiClient = authApiClient;
		_tokenStorage = tokenStorage;
		_vpn = new VpnSessionManager();
		ConnectCommand = new AsyncCommand(ConnectAsync, () => CanConnect && !_isConnecting);
		DisconnectCommand = new AsyncCommand(DisconnectAsync, () => CanDisconnect && !_isConnecting);
	}

	public string StatusText
	{
		get => _statusText;
		private set
		{
			if (_statusText != value)
			{
				_statusText = value;
				RaisePropertyChanged();
			}
		}
	}

	public bool CanConnect => !_isConnected;
	public bool CanDisconnect => _isConnected;

	public ICommand ConnectCommand { get; }
	public ICommand DisconnectCommand { get; }

	private async Task ConnectAsync()
	{
		_isConnecting = true;
		((AsyncCommand)ConnectCommand).RaiseCanExecuteChanged();
		((AsyncCommand)DisconnectCommand).RaiseCanExecuteChanged();
		StatusText = "Preparing...";
		try
		{
			var wintunOk = await _wintun.EnsureInstalledAsync();
			if (!wintunOk)
			{
				throw new InvalidOperationException("Wintun driver not found. Please install the driver.");
			}
			StatusText = "Connecting...";
			var tokens = await _tokenStorage.LoadAsync();
			var profile = await _authApiClient.FetchProfileAsync(tokens.AccessToken);
			await _vpn.ConnectAsync(profile, tokens.Email, tokens.AccessToken);
			_isConnected = true;
			StatusText = "Connected";
		}
		catch (Exception ex)
		{
			StatusText = $"Error: {ex.Message}";
			_isConnected = false;
		}
		finally
		{
			_isConnecting = false;
			((AsyncCommand)ConnectCommand).RaiseCanExecuteChanged();
			((AsyncCommand)DisconnectCommand).RaiseCanExecuteChanged();
		}
	}

	private async Task DisconnectAsync()
	{
		_isConnecting = true;
		((AsyncCommand)ConnectCommand).RaiseCanExecuteChanged();
		((AsyncCommand)DisconnectCommand).RaiseCanExecuteChanged();
		StatusText = "Disconnecting...";
		await _vpn.DisconnectAsync();
		_isConnected = false;
		StatusText = "Disconnected";
		_isConnecting = false;
		((AsyncCommand)ConnectCommand).RaiseCanExecuteChanged();
		((AsyncCommand)DisconnectCommand).RaiseCanExecuteChanged();
	}

	private void RaisePropertyChanged([CallerMemberName] string? name = null)
	{
		PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
	}

	private sealed class AsyncCommand : ICommand
	{
		private readonly Func<Task> _execute;
		private readonly Func<bool> _canExecute;

		public AsyncCommand(Func<Task> execute, Func<bool> canExecute)
		{
			_execute = execute;
			_canExecute = canExecute;
		}

		public event EventHandler? CanExecuteChanged;

		public bool CanExecute(object? parameter) => _canExecute();

		public async void Execute(object? parameter) => await _execute();

		public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);
	}
}


