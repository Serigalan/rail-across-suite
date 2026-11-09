Rail-Across-Suite — portable Version für Windows
================================================

Starten
  Den Ordner an einen beliebigen Ort entpacken (auch auf einen USB-Stick) und
  Rail-Across-Suite.exe starten. Eine Installation ist nicht nötig, Rechte als
  Administrator ebenso wenig.

  Beim ersten Start fragt die App nach der Sprache, dem Ordner der
  Arbeitsdateien und dem Exportordner. Später öffnet das Zahnrad oben rechts
  auf der Startseite diese Einstellungen wieder.

Was wo liegt
  data\            Arbeitsdateien: Projektdatenbank, Punktwolken, rail-across.log
  export\          exportierte Dateien
  webview\         Speicher der Oberfläche (Arbeitskopien, Sprache)
  settings.json    die Einstellungen des Startbildschirms
  site\, mdbtools\ die Suite selbst und der MDB-Import — nicht verändern

  Ordner innerhalb dieses Ordners werden relativ gespeichert: der Ordner darf
  verschoben werden, auch auf ein Laufwerk mit anderem Buchstaben. Wer beim
  Start einen Ordner außerhalb wählt, nimmt dessen Inhalt beim Verschieben
  nicht mit.

  Sichern = diesen Ordner kopieren, während die App geschlossen ist.

Voraussetzungen
  Windows 10 oder 11 (64 Bit) mit der Microsoft-Edge-WebView2-Laufzeit. Sie
  gehört zu Windows 11 und zu aktuellem Windows 10. Fehlt sie, startet die App
  nicht: https://developer.microsoft.com/microsoft-edge/webview2/
  ("Evergreen Standalone Installer").

Windows-Warnung
  Das Programm ist nicht signiert. Beim ersten Start kann Windows melden
  "Der Computer wurde durch Windows geschützt": "Weitere Informationen" und
  dann "Trotzdem ausführen".


Rail-Across-Suite — portable version for Windows
================================================

Unpack the folder anywhere (a USB stick included) and start
Rail-Across-Suite.exe — no installation, no administrator rights. The first
start asks for the language, the folder of the working files and the export
folder; the gear at the top right of the start page opens these settings
again.

data\ holds the working files (project database, point clouds,
rail-across.log), export\ the exported files, webview\ the window's own
storage (working copies, language), settings.json the settings. Folders inside
this folder are kept relative to it, so it may move. To back up, copy the
folder while the app is closed.

Needs Windows 10 or 11 (64 bit) with the Microsoft Edge WebView2 runtime
(part of Windows 11 and of current Windows 10). The program is not signed:
on "Windows protected your PC" choose "More info", then "Run anyway".
