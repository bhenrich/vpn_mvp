using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using System.Windows.Input;
using App.Services;

namespace App.ViewModels;

public class LoginViewModel : INotifyPropertyChanged
{
	private string _email = string.Empty;
	private string _password = string.Empty;
	private string _errorMessage = string.Empty;
	private bool _isBusy;

	private readonly AuthApiClient _authApiClient;
	private readonly TokenStorage _tokenStorage;
	private readonly Action _onSuccess;

	public event PropertyChangedEventHandler? PropertyChanged;

	public LoginViewModel(AuthApiClient authApiClient, TokenStorage tokenStorage, Action onSuccess)
	{
		_authApiClient = authApiClient;
		_tokenStorage = tokenStorage;
		_onSuccess = onSuccess;
		LoginCommand = new AsyncCommand(LoginAsync, () => CanLogin && !_isBusy);
	}

	public string Email
	{
		get => _email;
		set
		{
			if (_email != value)
			{
				_email = value;
				RaisePropertyChanged();
				RaisePropertyChanged(nameof(CanLogin));
				((AsyncCommand)LoginCommand).RaiseCanExecuteChanged();
			}
		}
	}

	public string Password
	{
		get => _password;
		set
		{
			if (_password != value)
			{
				_password = value;
				RaisePropertyChanged();
				RaisePropertyChanged(nameof(CanLogin));
				((AsyncCommand)LoginCommand).RaiseCanExecuteChanged();
			}
		}
	}

	public bool CanLogin => !string.IsNullOrWhiteSpace(Email) && !string.IsNullOrWhiteSpace(Password);

	public string ErrorMessage
	{
		get => _errorMessage;
		private set
		{
			if (_errorMessage != value)
			{
				_errorMessage = value;
				RaisePropertyChanged();
			}
		}
	}

	public ICommand LoginCommand { get; }

	private async Task LoginAsync()
	{
		try
		{
			_isBusy = true;
			((AsyncCommand)LoginCommand).RaiseCanExecuteChanged();
			ErrorMessage = string.Empty;
			var result = await _authApiClient.LoginAsync(Email, Password);
			await _tokenStorage.SaveAsync(result.AccessToken, result.RefreshToken, Email);
			_onSuccess();
		}
		catch (Exception ex)
		{
			ErrorMessage = ex.Message;
		}
		finally
		{
			_isBusy = false;
			((AsyncCommand)LoginCommand).RaiseCanExecuteChanged();
		}
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


