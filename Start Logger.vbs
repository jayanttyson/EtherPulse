Set WshShell = CreateObject("WScript.Shell")
Set FSO = CreateObject("Scripting.FileSystemObject")
currentDir = FSO.GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = currentDir

If FSO.FileExists(FSO.BuildPath(currentDir, "EtherPulse.exe")) Then
    WshShell.Run Chr(34) & FSO.BuildPath(currentDir, "EtherPulse.exe") & Chr(34), 0, False
ElseIf FSO.FileExists(FSO.BuildPath(currentDir, "LanCableLogger.exe")) Then
    WshShell.Run Chr(34) & FSO.BuildPath(currentDir, "LanCableLogger.exe") & Chr(34), 0, False
ElseIf FSO.FileExists(FSO.BuildPath(currentDir, "target\release\ether-pulse.exe")) Then
    WshShell.Run Chr(34) & FSO.BuildPath(currentDir, "target\release\ether-pulse.exe") & Chr(34), 0, False
ElseIf FSO.FileExists(FSO.BuildPath(currentDir, "target\release\lan-cable-logger.exe")) Then
    WshShell.Run Chr(34) & FSO.BuildPath(currentDir, "target\release\lan-cable-logger.exe") & Chr(34), 0, False
End If
