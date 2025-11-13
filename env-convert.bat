@echo off
REM Recursively find every env.example and copy to .env if not present

for /r %%D in (env.example) do (
    set "envfile=%%~dpD.env"
    set "examplefile=%%D"
    call :check_and_copy
)
goto :eof

:check_and_copy
REM Need delayed expansion for variable expansion inside loop
setlocal enabledelayedexpansion
if not exist "!envfile!" (
    copy "!examplefile!" "!envfile!"
    echo Created !envfile!
)
endlocal
goto :eof
