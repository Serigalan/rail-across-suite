# Betrieb

Rail-Across-Suite läuft auf **einem** Server: die Startseite der Suite, die Apps der Module, der Projektserver
mit der einen Anmeldung für alle, die Dienste der Module. Alles dafür liegt hier: eine Konfigurationsdatei, ein
paar Skripte und die Vorlagen für systemd und Caddy. Dienste: `rail-across-server`, `rail-across-optimizer`,
`rail-across-cloudjobs` und die tägliche Sicherung — je Instanz, siehe *Mehrere Instanzen*.

| Datei | |
|---|---|
| `rail-across.env.example` | Konfiguration, kommt nach `/etc/rail-across/rail-across.env` |
| `setup.sh` | einmalige Einrichtung (darf wiederholt werden) |
| `deploy.sh` | nach jedem Update: Abhängigkeiten, Build, Sicherung der DB, Neustart, Health-Check |
| `release.sh` | gibt einen Stand für die Produktion frei: Tag `prod-JJJJ-MM-TT`, nur bei grüner CI |
| `monitor.sh` | Überwachung aller Instanzen der Maschine (Timer `rail-across-monitor.timer`) |
| `templates/` | systemd-Units, Caddy-Site; `@RA_…@` wird aus der Konfiguration gefüllt |

## Aufbau

```
Internet ─▶ Caddy (TLS, Let's Encrypt)
             ├─ /            die Startseite der Suite (apps/home)           ┐ statisch, aus dist/
             ├─ /layout/     das Layout-Tool (apps/open-layout-tool)        ┘ (RA_SITE_DIR)
             ├─ /api/*       rail-across-server     Node: Anmeldung, Nutzer, Projekte, Varianten,
             │                                       Revisionen (core 1.0), Punktwolken; SQLite
             ├─ /optimizer/* rail-across-optimizer  Python: Optimierung, MDB, Gelände   ┐ nur mit
             └─ /layout/data/km*   Km-Linien aus dist/                                  ┘ Sitzung

rail-across-cloudjobs (ohne Port): bereitet hochgeladene Punktwolken auf und rechnet lange Läufe darauf
```

**Eine Anmeldung.** Startseite und Module liegen unter einer Adresse und sprechen mit einem Projektserver: die
Sitzung (Cookie `ra_session`) gilt überall. Angemeldet, das Passwort geändert und abgemeldet wird auf der
Startseite (`#/sign-in`, `#/password`); ein Modul ohne Sitzung schickt dorthin und bekommt den Nutzer zurück.

Beide Dienste hören nur auf `RA_BIND` (Standard `127.0.0.1`), nie öffentlich. Caddy fragt für `/optimizer/*` und
die Km-Linien per `forward_auth` beim Projektserver (`/api/me`), ob die Anfrage eine gültige Sitzung trägt.

## Neuer Server

Voraussetzungen: Debian 12 oder Ubuntu 22.04/24.04, root, ein DNS-Eintrag für den Hostnamen auf diesen Server,
Ports 80 und 443 frei.

```bash
git clone https://github.com/Serigalan/rail-across-suite.git /opt/rail-across-suite
mkdir -p /etc/rail-across
cp /opt/rail-across-suite/deploy/rail-across.env.example /etc/rail-across/rail-across.env
$EDITOR /etc/rail-across/rail-across.env       # mindestens RA_DOMAIN
/opt/rail-across-suite/deploy/setup.sh
```

Die NTv2-Gitter sind nicht im Repository: `setup.sh` und `deploy.sh` holen sie mit `node tools/data.mjs` des
Layout-Tools (Version und SHA-256 in `apps/open-layout-tool/data.json`). Die Km-Linien gibt es nur für Tests und
den eigenen Betrieb; wer sie hat, legt sie nach `apps/open-layout-tool/tiles/`.

`setup.sh` installiert Node 22 (NodeSource), Python-venv, `mdbtools` und Caddy, legt den Dienstnutzer
`rail-across` und `/var/lib/rail-across` an, kopiert die NTv2-Gitter dorthin, wo PROJ sie findet, baut die Site,
installiert den Optimierer (editierbar, liest `packages/core/constraints` direkt), richtet die systemd-Units und
die Caddy-Site ein und legt den ersten Admin an. Dessen Startpasswort steht danach in
`/var/lib/rail-across/admin-start-password.txt`; es wird bei der ersten Anmeldung auf der Startseite geändert,
die Datei danach löschen. Weitere Nutzer legt ein Admin an (vorerst in der Verwaltung des Layout-Tools).

## Update

```bash
cd /opt/rail-across-suite && deploy/deploy.sh --pull
```

Installiert npm- und pip-Abhängigkeiten nur, wenn sich ihre Lock-/Projektdatei geändert hat, baut die Site neben
`dist/` und gleicht sie dann hinein ab (die Seite ist während des Builds nie leer): je Modul seine App unter
seinem Pfad (`MODULE_APPS` in `lib.sh`), dann die Startseite an die Wurzel. Danach eine Kopie der Datenbank
(`backups/rail-across-JJJJ-MM-TT-predeploy-HHMMSS.sqlite`), Neustart der Dienste, Prüfung von `/api/health` und
`/health`. Nur prüfen: `deploy/deploy.sh --check-only`. Welcher Stand läuft: `$RA_DATA/.deploy-state/ref` und
`commit`.

Nach einer Änderung an der Konfiguration oder an `templates/`: `setup.sh` erneut ausführen.

npm 11 führt Installationsskripte nur aus, wenn `allowScripts` in der `package.json` an der Wurzel sie erlaubt;
der Build braucht das von esbuild.

## Mehrere Instanzen: Test und Produktion

Auf einer Maschine laufen beliebig viele Instanzen nebeneinander, jede mit eigener Konfiguration, eigenem
Checkout, Nutzer, Daten, Ports und Units. `RA_INSTANCE` gibt ihr den Namen: `RA_INSTANCE=test` → Units
`rail-across-test-server` …, Nutzer `rail-across-test`, Daten `/var/lib/rail-across-test`. Ohne Namen gelten die
einfachen Namen. Alle Skripte nehmen die Konfiguration mit `--config <datei>`. `setup.sh` bricht ab, wenn eine
Unit des gewünschten Namens zu einem anderen Checkout gehört oder eine andere Instanz schon dieselbe Datenbank
oder dieselben Ports benutzt. Liegt der Checkout unter einem geschlossenen Verzeichnis (`/root`), gibt
`setup.sh` dem Dienstnutzer per ACL nur das Durchgangsrecht.

**Release-Weg.** Eine Instanz mit `RA_TRACK=main` (Test) zieht mit `--pull` jeden Stand des ausgecheckten
Zweigs. Eine mit `RA_TRACK=tags` (Produktion) nimmt nur Tags `prod-…`: `deploy/release.sh` legt den nächsten
freien Tag `prod-JJJJ-MM-TT` an und pusht ihn, nur wenn die CI für den Commit grün ist; auf der Produktion dann
`deploy.sh --config … --ref prod-JJJJ-MM-TT`. **Zurück**: `deploy.sh --config … --ref <voriger Tag>` —
Migrationen laufen nur vorwärts, hat der neue Stand das Schema geändert, vorher die Sicherung zurückspielen.

## Auf dieser Maschine

Seit 2026-10-30, unter `https://railacross.open-layout-tool.org/`:

| | |
|---|---|
| Konfiguration | `/etc/rail-across/rail-across.env` |
| Checkout | `/opt/rail-across-suite` (folgt `dev`) |
| Units, Nutzer | `rail-across-*`, `rail-across` |
| Ports (an 172.18.0.1) | 8789 (Projektserver), 8101 (Optimierer) |
| Daten | `/var/lib/rail-across` |
| Site im Caddy | `/opt/rail-across-suite/dist` → `/srv/railacross` |
| Node | `/usr/local/bin/node` (`RA_NODE`: das Node in roots nvm darf der Dienstnutzer nicht lesen) |

Daneben laufen die beiden Instanzen des früheren Open-Layout-Tools aus dem Repository `open-layout-tool`
(online.open-layout-tool.org, db-ec.open-layout-tool.org; Units `olt-*`, `olt-prod-*`, Ports 8787/8099 und
8788/8100). Sie werden nicht umgezogen; ihre Skripte und ihre Überwachung (`olt-monitor`) liegen dort.

**Caddy** ist hier der Container `osrd-caddy` eines anderen Projekts (`/root/osrd/deploy/`), also
`RA_CADDY=external`: `setup.sh` schreibt die Site nach `/etc/open-layout-tool/caddy/rail-across.caddy`, ein
Verzeichnis, das der Container als `/etc/caddy/olt/` einhängt und importiert, und lädt Caddy neu. Die Site selbst
ist als `/srv/railacross` eingehängt (`docker-compose.deploy.yml`). Ein neu eingehängtes Verzeichnis braucht
einen neu erzeugten Container: Zeile ergänzen, dann `/root/osrd/deploy/osrd-deploy up -d --no-deps caddy`
(wenige Sekunden ohne Caddy, für alle Seiten). **Achtung:** OSRDs eigenes `setup.sh` schreibt das Caddyfile neu
und verliert dabei die `import`-Zeile — danach wieder anfügen und `docker restart osrd-caddy`.

## Daten und Sicherung

| Was | Wo (Standard) |
|---|---|
| Datenbank | `/var/lib/rail-across/rail-across.sqlite` (SQLite, WAL) |
| tägliche Kopie 03:30 | `/var/lib/rail-across/backups/rail-across-JJJJ-MM-TT.sqlite`, 30 Tage |
| Gelände-Kacheln (Cache) | `/var/lib/rail-across/terrain-cache` |
| NTv2-Gitter für PROJ | `/var/lib/rail-across/share/proj/` |
| Punktwolken | `/var/lib/rail-across/clouds/<Projekt>/<Wolke>/` (`RA_CLOUDS`), **nicht** in der täglichen Kopie |

Dazu vor jedem Neustart durch `deploy.sh` eine Kopie `rail-across-JJJJ-MM-TT-predeploy-HHMMSS.sqlite`. Gesichert
wird nur auf dieser Maschine. Wiederherstellen: `systemctl stop rail-across-server rail-across-cloudjobs`,
Sicherung über `rail-across.sqlite` kopieren (`-wal`/`-shm` daneben löschen, Eigentümer der Dienstnutzer),
wieder starten.

Admin von Hand anlegen:
`cd server && RA_SERVER_DB=/var/lib/rail-across/rail-across.sqlite runuser -u rail-across -- node bin/rail-across-server.mjs create-admin <login> [Name]`

## Überwachung

`deploy/monitor.sh <rail-across.env> …` prüft je Instanz: laufen die Dienste und der Sicherungs-Timer,
antworten Projektserver und Optimierer, antwortet `https://<domain>/api/health` von außen, ist das Zertifikat
noch mindestens 14 Tage gültig, sind mindestens 20 GB frei. Eingerichtet mit
`deploy/monitor.sh --install /etc/rail-across/rail-across.env` läuft es alle 5 Minuten
(`rail-across-monitor.timer`, Skript nach `/usr/local/lib/rail-across/` kopiert — nach Änderungen `--install`
wiederholen). Ein Fehler macht `rail-across-monitor.service` *failed*. Benachrichtigen kann es, sobald in der
Unit `RA_MONITOR_NOTIFY` steht (ein Befehl, der die Befunde auf stdin bekommt).

## Punktwolken

Eine hochgeladene Wolke liegt in `clouds/<Projekt>/<Wolke>/`: die Rohdatei, bis sie aufbereitet ist
(`raw.part`), die Kacheln der fünf Detailstufen (`tiles-L0.bin` … `tiles-L4.bin`) und ihre Indizes. Hochgeladen
wird in Stücken von 8 MB über `rail-across-server`, aufbereitet von `rail-across-cloudjobs` — höchstens
`RA_CLOUDJOBS_PARALLEL` Aufträge zugleich, je in einem eigenen Prozess; daneben die langen Läufe (Lichtraum,
Schienenverfolgung), höchstens `RA_CLOUDRUNS_PARALLEL`. Je Projekt höchstens 25 GB; vor jedem Hochladen prüft
der Server diese Quote und den freien Platz. Die tägliche Kopie enthält nur die Datenbank; wer Wolken sichern
will, kopiert `clouds/` gesondert. Was die Detailstufen sind:
[pointCloud](../apps/open-layout-tool/docs/modules/pointCloud.md).

## Lokale Entwicklung

Die Startseite (`apps/home`) und das Layout-Tool (`apps/open-layout-tool`) leiten mit `npm run dev` `/api` an
`OLT_API_TARGET` (Standard `http://127.0.0.1:8787`) weiter, das Layout-Tool `/optimizer` an
`OLT_OPTIMIZER_TARGET` (Standard `http://127.0.0.1:8099`), wie Caddy im Betrieb. Dienste lokal:

```bash
npm ci && cd server && RA_SERVER_INSECURE_COOKIE=1 RA_SERVER_DB=/tmp/rail-across.sqlite node bin/rail-across-server.mjs serve
python3 -m venv .venv && .venv/bin/pip install -c services/optimizer/requirements.lock -e 'services/optimizer[terrain]' && .venv/bin/olt-optimizer-serve
```

Angemeldet wird auf der Startseite; das Layout-Tool allein (`npm run dev` dort) sagt, dass die Anmeldung dort
liegt, denn es steht dann selbst an der Stelle der Startseite.
