@echo off
rem Starts ffx-rust.exe on the game folder (README.txt).
rem Give the game folder as an argument, drag it onto this file, or put
rem this folder's files into the game folder (the one with FFX.exe).
start "" "%~dp0ffx-launch.exe" --rust %*
