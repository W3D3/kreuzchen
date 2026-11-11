# Kreuzchen installieren

Kreuzchen läuft ohne Internetverbindung. Die Installationsdateien finden Sie auf der Seite „Releases" des Projekts. Die Programme sind nicht signiert, deshalb zeigen Windows und macOS beim ersten Start eine Warnung. Das ist normal.

## Windows

1. Laden Sie die Datei mit der Endung `.msi` herunter.
2. Doppelklicken Sie auf die Datei.
3. Erscheint die blaue Meldung „Der Computer wurde durch Windows geschützt" (SmartScreen), klicken Sie auf **Weitere Informationen**.
4. Klicken Sie auf **Trotzdem ausführen**.
5. Folgen Sie dem Installationsassistenten. Danach finden Sie Kreuzchen im Startmenü.

<!-- Screenshot: SmartScreen-Meldung mit „Weitere Informationen" -->
<!-- Screenshot: SmartScreen-Meldung mit „Trotzdem ausführen" -->

## macOS

1. Laden Sie die Datei mit der Endung `.dmg` herunter und öffnen Sie sie.
2. Ziehen Sie **Kreuzchen** in den Ordner **Programme**.
3. Öffnen Sie den Ordner **Programme**, klicken Sie mit der rechten Maustaste auf **Kreuzchen** und wählen Sie **Öffnen**. Bestätigen Sie mit **Öffnen**.
4. Falls das nicht klappt: Öffnen Sie **Systemeinstellungen → Datenschutz & Sicherheit**, scrollen Sie nach unten und klicken Sie bei der Meldung zu Kreuzchen auf **Trotzdem öffnen**.

Das ist nur beim ersten Start nötig.

<!-- Screenshot: Rechtsklick → Öffnen -->
<!-- Screenshot: Datenschutz & Sicherheit → Trotzdem öffnen -->

## Linux

Variante AppImage:

1. Laden Sie die Datei mit der Endung `.AppImage` herunter.
2. Machen Sie die Datei ausführbar: `chmod +x Kreuzchen*.AppImage`
3. Starten Sie die Datei per Doppelklick oder mit `./Kreuzchen*.AppImage`.

Variante Debian/Ubuntu:

1. Laden Sie die Datei mit der Endung `.deb` herunter.
2. Installieren Sie sie mit `sudo apt install ./Kreuzchen*.deb`.

## Fragebögen hinzufügen (für Entwicklung und Support)

Fragebögen sind YAML-Dateien im Ordner `definitionen` im Datenordner der App:

| System  | Ordner                                                           |
|---------|------------------------------------------------------------------|
| Windows | `%APPDATA%\io.github.w3d3.kreuzchen\definitionen`                |
| macOS   | `~/Library/Application Support/io.github.w3d3.kreuzchen/definitionen` |
| Linux   | `~/.local/share/io.github.w3d3.kreuzchen/definitionen`           |

1. Kopieren Sie die YAML-Datei in diesen Ordner.
2. Starten Sie Kreuzchen neu.

In der App zeigt das Halten der **Alt**-Taste technische Angaben an, darunter den Pfad dieses Ordners. Mit **Strg+Umschalt+D** lassen sie sich dauerhaft ein- und ausblenden.

<!-- Screenshot: Startseite mit eingeblendetem Ordnerpfad (Alt gedrückt) -->
