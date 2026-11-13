# Zweck

rail-across-1.0 beschreibt ein Eisenbahnprojekt in **einer** JSON-Datei (`.railacross.json`): seine
Infrastruktur und seinen Betrieb. Es ist die Grundlage des Datenmodells der Rail-Across-Suite: Was hier steht,
ist das, was die Werkzeuge halten, austauschen und versionieren.

- **infra-data-1.0** hält den Planungsstand der Infrastruktur: das Gleisnetz mit seiner Trassierung und seinen
  Höhen, Weichen und Gleisenden, Bezugsdaten wie Kilometrierungslinien und Betriebsstellen, die Halteplätze —
  und darauf die Layer der Fachgebiete: Bahnsteige und Ingenieurbauwerke (`civil`), die Leit- und
  Sicherungstechnik (`signalling`), die Oberleitung (`electrification`).
- **operations-1.0** hält den Betrieb: die Fahrzeuge (`vehicles`) und die Fahrpläne mit ihren Zügen,
  Zuggattungen, Zugbildungen, Anschlüssen und Wenden, Verspätungsszenarien und Kapazitätsuntersuchungen
  (`timetables`).

Die Infrastruktur wird in Varianten geplant; der Betrieb gehört zu keiner Variante und nennt die Infrastruktur
nur über Kennungen. So lässt sich ein Fahrplan auf jede Variante legen.

Das Format folgt sieben Grundsätzen:

- **Entwurfswerte statt Polylinien.** Ein Gleis wird als das gespeichert, was entworfen wurde: Geraden,
  Kreisbögen und Übergangsbögen mit Länge, Radius und Überhöhung. Jede gezeichnete Koordinate wird daraus
  abgeleitet.
- **Ebene Koordinaten, eine Ebene je Gleis.** Trassierungen werden in projizierten Ebenen entworfen und vermessen
  (DB_REF/Gauß-Krüger, ETRS89/UTM, historische Lagesysteme). Ein Gleis behält die Ebene, in der es entworfen
  wurde. Wechselt die Ebene, treffen sich zwei Gleise an einem Verbinder.
- **Abgeleitetes wird nicht gespeichert.** Stationen, Polylinien, Weichensymbole, Bahnsteigflächen, die
  Ausdehnung von Freimeldeabschnitten, der Verlauf eines Fahrdrahts und die Bauzustände folgen aus dem Rest. Was
  nicht gespeichert ist, kann ihm nicht widersprechen.
- **Feste Kennungen, geprüfte Verweise.** Jedes Objekt hat eine eigene Kennung, eine UUID. Ein Verweis nennt
  diese Kennung, nie einen Namen oder eine Listenposition, und jeder Verweis muss sein Objekt finden. Das macht
  das Zusammenführen zweier Stände Objekt für Objekt und Feld für Feld möglich.
- **Ein Modell.** Was gespeichert wird, ist das, womit gearbeitet wird: Eine Betriebsstelle ist ein Objekt mit
  Kennung, kein Name, der an jedem Gleis wiederholt wird; ein Bauzustand ist ein Tag, kein Eintrag in einem
  Verzeichnis.
- **Versionen, die zählen.** Jede Änderung eines Schemas erhöht die Version seiner Domäne. Ein Leser bringt eine
  ältere Datei Schritt für Schritt auf seinen Stand.
- **Unbekanntes bleibt erhalten.** Eine Domäne oder ein Feld, das ein Leser nicht kennt, behält er, wie es kam,
  und schreibt es zurück.

# Aufbau einer Datei

## Die Datei und ihre Teile

<!-- cols: 20%, 24%, 1fr -->
| Name | Endung | Inhalt |
|---|---|---|
| rail-across-1.0 | `.railacross.json` | die Datei: das Projekt, das erzeugende Programm und der Zeitpunkt, die Teile, die Blobs |
| infra-data-1.0 | — (ein Teil der Datei) | die Domäne `core` und die Layer `civil`, `signalling`, `electrification` |
| operations-1.0 | — (ein Teil der Datei) | die Domänen `vehicles` und `timetables` |

Eine Datei hält einen Teil oder beide. Ein Projektserver hält sie getrennt — die Infrastruktur in Revisionen je
Variante, den Betrieb in eigenen Revisionen —, eine Datei führt sie für den Austausch zusammen.

## Die Hülle

```json
{
  "format": "rail-across",
  "version": "1.0",
  "projectId": "5dbd8fed-f8dc-4995-8934-9d71be69cae0",
  "generator": { "name": "Rail-Across-Suite", "version": "0.9.0" },
  "savedAt": "2026-11-13T09:30:00Z",
  "infraData": {
    "format": "infra-data", "version": "1.0",
    "domains": {
      "core": {
        "version": "4.0", "schema": "sha256:…",
        "fields": { … }, "collections": { … }
      },
      "civil":           { "version": "3.0", "dependsOn": ["core"], … },
      "signalling":      { "version": "4.0", "dependsOn": ["core"], … },
      "electrification": { "version": "3.0", "dependsOn": ["core"], … }
    }
  },
  "operations": {
    "format": "operations", "version": "1.0",
    "domains": {
      "vehicles":   { "version": "1.0", … },
      "timetables": { "version": "1.0", "dependsOn": ["vehicles"], … }
    }
  },
  "blobs": { "sha256:…": … }
}
```

<!-- cols: 26%, 1fr -->
| Teil | Inhalt |
|---|---|
| `format`, `version` | immer `rail-across` und `1.0` |
| `projectId` | die Kennung des Projekts, eine UUID |
| `generator`, `savedAt` | Programm und Version, die die Datei geschrieben haben, und wann (ISO 8601, UTC) |
| `infraData` | der Teil infra-data-1.0: Format, Version und seine Domänen; `core` ist Pflicht |
| `operations` | der Teil operations-1.0: Format, Version und seine Domänen |
| `blobs` | große Datenmengen (Messpunkte, Punkte von Kilometrierungslinien), je unter der Prüfsumme ihres Inhalts, gemeinsam für beide Teile |

Die Datei führt keine Schemas mit: Eine Domäne nennt ihr Schema über seine Prüfsumme, und ein Leser kennt die
Versionen, die er liest.

## Domänen und Layer

Eine **Domäne** ist ein Fachgebiet mit eigenem Schema und eigener Version. Sie hat eigene Felder (`fields`) und
Sammlungen (`collections`). Ein **Layer** ist eine Domäne von infra-data, die auf dem Kern aufsetzt: Er nennt
`core` unter `dependsOn` und verweist nur in sich selbst und in den Kern. Jeder Layer wird in seinem eigenen
Werkzeug bearbeitet.

<!-- cols: 16%, 22%, 1fr -->
| Teil | Domäne | Inhalt |
|---|---|---|
| infra-data-1.0 | `core` 4.0 | das Netz: Gleise, Weichen und Verbinder, Gleisenden (Prellböcke, Betrachtungsgrenzen), Kilometrierungslinien, Betriebsstellen, Halteplätze, benannte Gleisfolgen. Die Grundlage aller anderen |
| | `civil` 3.0 | Bahnsteige und Ingenieurbauwerke (Eisenbahnüberführungen, Stützwände, Lärmschutzwände), in Punktwolken gemessene Achsen, Bestandsachsen, Schriftfeld, Projektbild |
| | `signalling` 4.0 | Signale mit logischen Signalen, Gleisfreimeldung, Weichenelemente, Fahrstraßen, Durchrutschwege, Flankenschutz, Zugbeeinflussung, Bahnhofsgrenzen, Fahrwege — nach PlanPro exportierbar |
| | `electrification` 3.0 | Masten, Aufhängepunkte, Streckentrennungen und die Fahrdrähte mit ihrer Spannung |
| operations-1.0 | `vehicles` 1.0 | die Fahrzeuge mit ihrem Rollmaterial als RailJSON |
| | `timetables` 1.0 | Fahrpläne, Züge, Zuggattungen, Zugbildungen, Anschlüsse, Wenden, Verspätungsszenarien, Untersuchungen |

Die Domänen von operations nennen die Infrastruktur über Kennungen: Betriebsstellen, Gleisenden, Gleise,
Fahrwege. Diese Verweise werden geprüft, wo der Betrieb auf die Infrastruktur gelegt wird — auf den Teil
infra-data derselben Datei oder eine Variante auf einem Server.

## Sammlungen und Objekte

- Eine Sammlung ist eine Liste von Objekten mit `id`, **sortiert nach `id`**. Derselbe Stand ergibt so dieselben
  Bytes und dieselbe Prüfsumme.
- Folgen innerhalb eines Objekts (`elements`, `heights`, die Punkte eines Zugs) behalten ihre Reihenfolge, sie
  haben eine Bedeutung.
- Jedes Objekt erlaubt nur die Properties, die sein Schema nennt (`additionalProperties: false`).
- **Kennungen sind immer UUIDs in Kleinbuchstaben**, eindeutig in ihrer Sammlung — auch wo ein Import sie aus
  etwas bildet, das er gelesen hat (eine Kilometrierungslinie aus ihrer Streckennummer, einen Bahnhof aus seinem
  Namen): dann als daraus gebildete UUID.

# Grundregeln

## Prüfsummen

Eine Domäne nennt ihr Schema über `schema`: den SHA-256 seines **kanonischen JSON** nach RFC 8785 (JSON
Canonicalization Scheme), geschrieben als `sha256:` und 64 Hexziffern. Die Prüfsumme dient nur der
Unversehrtheit — welche Version eine Domäne hat, sagt ihre `version`. Jeder Eintrag in `blobs` muss genau die
Prüfsumme seines Inhalts tragen.

## Verweise

Ein Verweis ist die Kennung seines Ziels. Im Schema trägt das Property die Anmerkung `x-target`, die die Sammlung
nennt, in die es zeigt, z. B. `core.tracks` oder `signalling.routes`, und sein Name endet auf `Id` (`Ids` für eine
Liste). Jeder solche Verweis muss ein Objekt der genannten Sammlung finden; eine Datei, deren Verweis ins Leere
zeigt, wird nicht angenommen (siehe *Prüfung beim Einlesen*). Gleisenden werden als Paar aus Gleis und Ende
genannt (`TrackEnd`: `trackId` und `endpoint` `BEGIN` oder `END`).

## Orte auf dem Gleis

Wo ein Objekt steht, ist **ein** Wert (`TrackLocation`) mit einem Anker:

- `anchor: "station"` — das Objekt folgt dem Gleis: `trackId`, `station` (m ab Gleisanfang), `side`
  (`left`/`right`, gesehen in Stationierungsrichtung), `offset` (m von der Achse), `direction` (`up` mit der
  Stationierung, `down` dagegen). Wird ein Gleis geteilt, verbunden oder umgedreht, werden diese Orte mitgeführt.
- `anchor: "plane"` — das Objekt behält seinen Platz in der Ebene: `easting`, `northing`, `epsg`, optional
  `bearing`. So bleibt ein Objekt erhalten, dessen Gleis entfernt wurde.

Ein Abschnitt eines Gleises ist ein `TrackRange` (`trackId`, `startStation`, `endStation`, `side`), eine
Gleisfolge ein `TrackPath` (`trackIds` in Fahrtrichtung; die Richtung je Gleis folgt daraus, wo sie
aneinanderstoßen).

## Lebensdauer und Bauzustände

Jedes Objekt kann eine **Lebensdauer** tragen: die Tage, an denen es in Betrieb ist.

<!-- cols: 46%, 1fr -->
| Lebensdauer | Bedeutung |
|---|---|
| `{ "from": "2027-05-01" }` | neu ab dem 1. Mai 2027 |
| `{ "from": "2027-05-01", "to": "2031-12-12" }` | vorübergehend, vom 1. Mai 2027 bis zum 12. Dezember 2031 |
| `{ "from": "open" }` | neu, der Tag steht noch nicht fest |
| `{ "to": "open" }` | wird zurückgebaut, der Tag steht noch nicht fest |
| keine | besteht und bleibt |

`from` ist der erste Tag in Betrieb, `to` der letzte. Ohne `from` besteht das Objekt heute, ohne `to` bleibt es.
Ein Objekt, das weder neu gebaut noch zurückgebaut wird, trägt **keine** Lebensdauer.

Die **Bauzustände** werden nicht gespeichert, sie folgen aus den Lebensdauern: der Bestand, ein Zustand für jeden
Tag, an dem etwas in oder außer Betrieb geht, und der Endzustand.

<!-- cols: 24%, 22%, 30%, 1fr -->
| Lebensdauer | im Bestand | an einem Tag T | im Endzustand |
|---|---|---|---|
| keine | ja | ja | ja |
| `from`: Tag F | nein | wenn F ≤ T | ja |
| `from`: `open` | nein | nein | ja |
| `to`: Tag L | ja | wenn T ≤ L | nein |
| `to`: `open` | ja | ja | nein |

Bei beiden Angaben müssen beide Bedingungen gelten. Der angezeigte Status folgt ebenso: mit `from` *neu*, mit `to`
allein *Rückbau*, sonst *Bestand*. Eine Weiche ohne eigene Lebensdauer zeigt, was ihre Gleise sagen.

## Blobs

Große Datenmengen stehen nicht im Objekt, sondern in `blobs`: die Punkte gemessener Achsen und Bestandsachsen
(`civil`), die Punkte einer Kilometrierungslinie, wo eine Quelle keine Trassierungselemente geliefert hat
(`core`). Das Objekt nennt den Blob über die Prüfsumme seines
kanonischen JSON. Gleiche Inhalte stehen so nur einmal in der Datei.

## Unbekanntes bleibt erhalten

Eine Domäne, die ein Leser nicht kennt, behält er ganz und schreibt sie unverändert zurück. Eine Domäne einer
späteren Nebenversion, als er kennt, liest er mit dem Schema der Version, die er kennt; Felder, die er nicht
kennt, behält er, wie sie kamen, und schreibt sie zurück. Eine Domäne einer späteren Hauptversion kann er nicht
lesen; er sagt das und lehnt die Datei ab.

# Das Netz (Domäne `core`)

## Gleise und Trassierungselemente

Ein Gleis (`tracks`) hat eine Ebene (`epsg`), in der alle seine Elemente angegeben sind, ein Höhenbezugssystem
(`heightEpsg`), die Folge seiner Trassierungselemente (`elements`) und seine Höhen (`heights`). Ein Bahnhofsgleis
(`trackType: "station"`) nennt seine Betriebsstelle (`operationalPointId`) und seine Gleisnummer, ein Streckengleis
(`trackType: "line"`) seine Strecke und welches Gleis der Strecke es ist (`side`: `regular` Richtungsgleis oder
`opposite` Gegenrichtungsgleis).

**Ein Element hat keine Station und keine Polylinie.** Die Station eines Punkts ist die laufende Summe der Längen
vom Gleisanfang. Die Geometrie folgt aus dem Anfangspunkt, der Richtung und den Entwurfswerten. Ein Element hat
keine eigene Ebene: Es liegt in der seines Gleises.

<!-- cols: 22%, 26%, 26%, 1fr -->
| Wert | `straight` (Gerade) | `arc` (Kreisbogen) | `transition` (Übergangsbogen) |
|---|---|---|---|
| `startNode`, `endNode` | `[E, N]` in m | `[E, N]` in m | `[E, N]` in m |
| `bearing` | Richtung am Anfang, Grad, von Gitternord im Uhrzeigersinn | ebenso | ebenso |
| `length` | m | m, Bogenlänge | m |
| `radius` | — | m, vorzeichenbehaftet: + Rechtsbogen | — |
| `r1`, `r2` | — | — | Radien am Anfang und Ende, vorzeichenbehaftet; `null` = gerades Ende |
| `transitionType` | — | — | `clothoid` oder `bloss` |
| `cant` | mm, an beiden Enden gleich | mm | — |
| `cantStart`, `cantEnd` | mm, wo die Überhöhung entlang des Elements wechselt | ebenso | mm, immer beide |
| `endBearing` | nur bei einem Knick | nur wenn abweichend | nur wenn abweichend |

Ein Element gibt seine Überhöhung als `cant` **oder** als `cantStart` und `cantEnd` an, nie beides. Eine fehlende
Überhöhung ist 0. Die Überhöhung ist vorzeichenbehaftet nach der überhöhten Schiene, in Stationierungsrichtung
gesehen. Dazu kommen die Entwurfsgeschwindigkeit (`designSpeed`, km/h), die Rolle bei Änderungen (`role`),
Begründungen zu Regelhinweisen (`reasons`) und an Weichenstrecken die Markierung `switchBranch`, `switchRoute`,
`switchId`. Ein als Weichenstrecke markiertes Element nennt seine Weiche.

## Höhen

Die Höhen sind ein Neigungsbruchpolygon, unabhängig von den Trassierungselementen: Punkte mit `station` (m ab
Gleisanfang) und `z` (m, Oberkante der nicht überhöhten Schiene), optional mit Ausrundungsradius `rv` und der
Länge `la`, die ihn bestimmt. Ein Punkt kann von einem anderen desselben Gleises abhängen (`depId`, `depMode`),
etwa in einer Weiche. Zwischen zwei Punkten ist die Neigung konstant.

## Weichen und Verbinder

Ein Objekt der Sammlung `switches` sitzt an einem Knoten. Seine **Ports** halten Gleisenden (`TrackEnd`). Welche
Ports es hat, legt `kind` fest:

<!-- cols: 26%, 18%, 1fr -->
| `kind` | Ports | Bedeutung |
|---|---|---|
| `switch` | `A`, `B1`, `B2` | Weiche: `A` Weichenanfang, `B1` Stammgleis, `B2` Zweiggleis |
| `crossing` | `A`, `B`, `C`, `D` | Kreuzung |
| `single_slip`, `double_slip` | `A`, `B`, `C`, `D` | einfache und doppelte Kreuzungsweiche |
| `link` | `A`, `B` | Verbinder: zwei Gleisenden, wo die Ebene wechselt oder ein umgebautes Stück anschließt |
| `stage_link` | `A`, `B1`, `B2` | Bauzustandsverbinder: das Gleis an `A` ist mit dem an `B1` verbunden, solange es in Betrieb ist, danach mit dem an `B2` |

Die Strecken einer Weiche sind die Elemente der angeschlossenen Gleise, die mit `switchId` und `switchRoute`
(`main`, `branch`) markiert sind. Die Form steht in `label` (z. B. `300 – 1:9`) und verweist auf einen Formkatalog,
`formVersion` nennt dessen Version. Das Symbol der Weiche wird nicht gespeichert.

## Betriebsstellen und Halteplätze

Eine **Betriebsstelle** (`operationalPoints`) ist ein Objekt mit fester Kennung: Bahnhof, Haltepunkt oder eine
andere Betriebsstelle, mit Name, RL100-Kürzel und UIC-Nummer. Sie wird **über ihre Kennung gefunden**: Gleise,
Bahnsteige, Signale, Halteplätze und die Punkte der Züge verweisen über ihre UUID auf sie; ihr Name steht nur bei
ihr. Umbenennen ändert genau ein Feld. Ihre Grenzen legt die Leit- und Sicherungstechnik fest
(`stationBoundaries`).

Ein **Halteplatz** (`stoppingPlaces`) ist, wo Züge halten: ein Abschnitt eines Gleises, im Bahnhof oder auf der
freien Strecke, für jeden Zug. Er gehört zum Kern, weil Leit- und Sicherungstechnik, Bahnsteige und Fahrpläne ihn
gleichermaßen brauchen. Ein **Personenhalt** (`passengerStop`) ist er, wo ein Bahnsteig an ihm liegt und ihn nennt
(Layer `civil`, `stoppingPlaceIds`).

## Kilometrierungslinien

Eine **Kilometrierungslinie** (`kmLines`) sind fremde Bezugsdaten: die Achse einer Strecke um das Projekt mit ihrer
Kilometrierung. Sie wird **in Trassierungselementen** gespeichert, wie ein Gleis: in einer Ebene (`epsg`), als
Läufe von Kilometersprung zu Kilometersprung (`runs`). Ein Lauf nennt seinen Kilometer am Anfang (`kmStart`, m)
und wie er entlang des Laufs zählt (`counting`: `up` steigend oder `down` fallend); der Kilometer eines Punkts ist
der Anfangswert plus oder minus die laufende Länge bis zu ihm.

Das ist weit kleiner als eine Polylinie mit einem Punkt alle paar Meter, und genau, wo die Quelle Elemente angibt —
die Streckenachsen der DB sind in Elementen entworfen wie die Gleise. Dieselbe Rechnung dient Gleisen und
Kilometrierungslinien. Für ein Gleis in einer anderen Ebene wird die Achse in diese umgerechnet. Wo eine Quelle nur
Punkte liefert, hält ein Lauf sie als Blob (`points`: `[Rechtswert, Hochwert, Kilometer]` je Punkt), bis er mit
Elementen angenähert ist.

## Weitere Sammlungen

- `endMarks`: Prellbock (`buffer_stop`) oder Betrachtungsgrenze (`boundary`) an einem freien Gleisende. Eine
  Grenze nennt die Geschwindigkeit, mit der Züge sie überqueren, und den kleinsten Abstand zweier Züge.
- `trackPaths`: benannte Gleisfolgen, für Längsschnitte und Tabellen der Trassierung.

# Bahnsteige und Bauwerke (Layer `civil`)

Der Layer hält, was im Gleisnetz steht, ohne zum Netz zu gehören:

- **Bahnsteige** (`platforms`) als `TrackRange` mit Kantenabstand, Höhe und Betriebsstelle. Ein Bahnsteig nennt die
  Halteplätze an ihm (`stoppingPlaceIds`), die dadurch Personenhalte sind.
- **Ingenieurbauwerke**: Eisenbahnüberführungen (`bridges`) mit Tragwerk, Feldern und lichter Höhe, Stützwände
  (`retainingWalls`) und Lärmschutzwände (`noiseBarriers`) mit Bauart, Abstand und Höhe — jeweils als Abschnitt eines
  Gleises. Wird das Gleis geteilt oder umgedreht, werden sie mitgeführt.
- **Gemessene Achsen** (`axisSurveys`, Punkte als ganze Millimeter in parallelen Listen, als Blob) und
  **Bestandsachsen** aus Vermessungen (`referenceAxes`, ein Punkt je Zentimeter, als Blob).
- Das Schriftfeld des Plans und das Projektbild (`fields`).

# Die Leit- und Sicherungstechnik (Layer `signalling`)

Der Layer bildet die Leit- und Sicherungstechnik für die Eisenbahnplanung ab. Seine Begriffe lassen sich auf
PlanPro 1.10 abbilden, damit ein Plan **nach PlanPro exportiert** werden kann; er folgt PlanPro nur so weit, wie
diese Verträglichkeit es verlangt, nicht in seinem Aufbau. Jeder Standort steht auf einem Gleis des Kerns.

- **Signale** (`signals`) sind physische Standorte, real oder fiktiv. Ein reales Signal trägt je Signalsystem ein
  **logisches Signal** (z. B. Ks-Hauptsignal, Sh-Signal) mit Schirm, Zusatzanzeigern und der signalisierten
  Geschwindigkeit je Fahrstraße. Tafeln ohne System (Ne, Lf, Ra, Zs …) sind Rahmen des Standorts selbst. Ein
  fiktives Signal markiert, wo eine Fahrstraße ohne Signal beginnt oder endet, etwa an einer Betrachtungsgrenze.
- **Begriffe** nennen ihren Schlüssel im Katalog, den PlanPro-Namen (`Hp_0`, `Ks_1`, `Zs_3`, `Lf_7`), mit Symbol,
  wo der Katalog eines verlangt: bei Zs 3 und Lf 7 die Geschwindigkeit in Zehnern km/h (`7` = 70 km/h).
- **Gleisfreimeldung**: Freimeldepunkte (`detectionPoints`) begrenzen Abschnitte (`trackSections`). Die Ausdehnung
  eines Abschnitts wird nicht gespeichert, sie folgt aus seinen Punkten über das Netz.
- **Weichenelemente** (`switchElements`) sind die Weichen des Kerns und Gleissperren, wie das Stellwerk sie kennt.
- **Fahrstraßen** (`routes`) führen von einem Start- zu einem Zielsignal über einen `TrackPath`, mit
  Fahrstraßenbilde- und Auflösezeit, wo sie von denen des Layers abweichen. Die Weichenlagen folgen aus der
  Gleisfolge. Dazu kommen **Durchrutschwege** (`overlaps`) und **Flankenschutz** (`flankProtection`).
- **Zugbeeinflussung**: punktförmig (`balises`: PZB-Magnete, Geschwindigkeitsprüfungen, ETCS-Balisen) und über
  Gleisabschnitte (`trainProtection`: PZB, LZB, ETCS mit Level).
- **Bahnhofsgrenzen** (`stationBoundaries`) und **Fahrwege** (`itineraries`) von Halteplatz zu Halteplatz.

Welche Signalsysteme, Signalarten, Begriffe und Einstellungen es gibt, legt nicht das Schema fest, sondern ein
**Katalog** nach DB Ril 301. Der Layer nennt ihn in `fields.catalogue` (`id`, `version`). Ein neues Signalsystem
ist ein neuer Katalogeintrag, kein neues Schema.

# Die Oberleitung (Layer `electrification`)

Der Layer hält die Oberleitung. **Alles ist auf ein Gleis** des Kerns bezogen.

- **Masten** (`masts`) stehen an einem Ort auf einem Gleis (Station, Seite, Abstand von der Achse): ein Einzelmast
  mit Ausleger, ein Joch oder ein Quertragwerk über mehrere Gleise.
- **Aufhängepunkte** (`suspensionPoints`) gehören zu einem Mast und stehen über einem Gleis: Jeder hält dort einen
  Fahrdraht, mit seiner Höhe und Seitenverschiebung. Ein Joch hält über jedem seiner Gleise einen.
- **Streckentrennungen** (`sectionInsulators`) sind Bauteile der Oberleitung, die zwei Stromkreise auf einem Gleis
  trennen: Sie nennen den Fahrdraht davor und den dahinter, in Stationierungsrichtung. Eine Schutzstrecke ist eine
  von ihnen.
- **Fahrdrähte** (`contactWires`) sind die Stromkreise. Ein Fahrdraht führt eine **Spannung**: `15000V`
  (15 kV 16,7 Hz), `25000V` (25 kV 50 Hz), `3000V`, `1500V`, `750V` oder `600V` (Gleichstrom), genannt wie OSRD die
  Traktionsart nennt.

Wo ein Fahrdraht verläuft, wird nicht gespeichert: Auf einem Gleis verläuft er von seinem ersten bis zu seinem
letzten führenden Punkt dort — den Aufhängepunkten und Streckentrennungen, die ihn nennen. Die Spannung über einem
Gleisabschnitt ist die des Fahrdrahts darüber. **Der Betrieb liest sie**: Ein Fahrzeug fährt über einem Abschnitt
elektrisch, dessen Spannung unter seinen Traktionsarten ist; anderswo braucht es einen eigenen Antrieb
(`thermal`), oder es kann dort nicht fahren.

# Der Betrieb (Teil operations-1.0)

## Fahrzeuge (Domäne `vehicles`)

Die Fahrzeuge sind eine eigene Domäne. Ein Fahrzeug (`vehicles`) hat den Schlüssel, unter dem es aus einem Katalog
kam (`key`), und sein **Rollmaterial** als RailJSON (`rollingStock`), dem Format von OSRD — im Objekt gehalten,
nicht als Blob: Ein Fahrzeug hat wenige Kilobyte, und im Objekt lassen sich zwei Stände Feld für Feld vergleichen
und zusammenführen. Das Schema prüft, was die Suite daraus liest — die RailJSON-Version, den Namen und die
Zugkraftkurven —, und übernimmt jedes weitere RailJSON-Property, wie es kommt, in den Einheiten von RailJSON
(m, kg, m/s, N).

Die **Traktionsarten** eines Fahrzeugs sind die Betriebsarten seiner Zugkraftkurven (`effort_curves.modes`): die
Spannungen, die es abnehmen kann, benannt wie die Spannungen der Fahrdrähte, und `thermal` für eigenen Antrieb. Sie
werden kein zweites Mal gespeichert. Die Fahrpläne nennen Fahrzeuge über ihre Kennung. Eine Datei fährt so mit den Fahrzeugen, die sie hält, nicht mit einem Katalog, der sich inzwischen
geändert haben kann; ein geändertes Fahrzeug ist eine neue Version der Domäne wie jede andere Änderung.

## Fahrpläne (Domäne `timetables`)

- Ein **Fahrplan** (`timetables`) sammelt Züge und sagt, was geschieht, wo sie sich begegnen (`dispatching`: nur die
  Konflikte zeigen oder die Züge nacheinander ins Netz geben, nach Abfahrt oder nach Gattung), und welche Teile der
  Sperrzeit der Betrieb vorgibt (`blockingTimes`).
- Ein **Zug** (`trains`) hat eine Nummer, eine Gattung, eine Zugbildung, eine Abfahrt in Sekunden ab Mitternacht des
  Fahrplantags (über 24:00 der nächste Tag), einen Fahrzeitzuschlag und seine **Punkte**: jeder eine Betriebsstelle
  (`operationalPointId`) oder ein Gleisende — Prellbock oder Betrachtungsgrenze (`endMarkId`) — **über seine
  UUID**, mit dem Gleis (`trackId`), ob er hält, seiner Haltezeit, einer Abfahrt und dem Fahrweg, den er zum
  nächsten Punkt bevorzugt (`itineraryId`).
- **Zuggattungen** (`trainKinds`) mit Rang und Farbe; **Zugbildungen** (`formations`) aus Fahrzeugen von vorn nach
  hinten (`vehicleId`), Wagenzuglast und Bremse.
- **Anschlüsse** (`connections`) und **Wenden** (`turnarounds`) zwischen Zügen.
- **Verspätungsszenarien** (`delayScenarios`) und **Kapazitätsuntersuchungen** (`studies`) nach UIC 406.

## Wie der Betrieb die Infrastruktur liest

Ein Zug findet seine Betriebsstellen, Gleisenden, Gleise und Fahrwege über ihre Kennungen in der Infrastruktur, auf
die er gelegt wird; wo einer Variante eine fehlt, lässt sich der Zug dort nicht auf sie legen, und ein Werkzeug sagt,
welche. Wo er elektrisch fährt, entscheiden die Spannungen der Fahrdrähte auf seinem Weg und die Traktionsarten
seiner Fahrzeuge.

# Einheiten und Genauigkeit

<!-- cols: 36%, 30%, 1fr -->
| Größe | Einheit | Genauigkeit |
|---|---|---|
| Koordinaten, Längen, Radien, Stationen | m | 0,1 µm (7 Nachkommastellen) |
| Höhen `z` | m | 0,1 mm (4 Nachkommastellen) |
| Richtungswinkel | °, von Gitternord im Uhrzeigersinn | 10⁻⁹ ° |
| Überhöhung | mm | 0,001 mm |
| Geschwindigkeiten | km/h; bei Fahrzeugen (RailJSON) m/s | wie angegeben |
| Zeiten im Betrieb | s | wie angegeben |
| Spannung | V, als Name | — |
| Kilometrierung | m | wie geliefert |

Positionen werden so fein gehalten, weil Werte beim erneuten Einlesen aneinandergereiht werden: Ein gerundeter Punkt
würde an einer Weiche einen Tangentenknick erzeugen, und ein gerundeter Richtungswinkel lässt ein langes Gleis
seitlich abwandern.

# Prüfung und Versionen

## Prüfung beim Einlesen

Ein Leser nimmt eine Datei erst nach einer Prüfung an:

1. Format und Version der Hülle und jedes Teils; anderes wird als `unsupported_version` abgelehnt.
2. Die Prüfsummen der Blobs.
3. Die Version jeder Domäne. Eine ältere Version wird auf den Stand des Lesers gebracht (unten); eine spätere
   Nebenversion wird gelesen, Unbekanntes bleibt erhalten; eine spätere Hauptversion wird mit diesem Hinweis
   abgelehnt.
4. Jede Domäne gegen ihr Schema, jede Kennung eine UUID und nur einmal in ihrer Sammlung.
5. Was ein Schema nicht ausdrücken kann: Die Tage einer Lebensdauer sind Tage, der erste liegt nicht nach dem
   letzten; ein als Weichenstrecke markiertes Element nennt seine Weiche; eine Traktionsart eines Fahrzeugs ist genau dann elektrisch, wenn
   sie eine Spannung ist; **jeder Verweis findet sein Objekt** —
   innerhalb des Teils, in dem er steht; die des Betriebs in die Infrastruktur, wo der Betrieb auf sie gelegt wird.

Fehler der Schritte 2, 4 und 5 werden als `invalid_payload` mit den gefundenen Stellen gemeldet, z. B.
`/infraData/domains/signalling/collections/switchElements/0/switchId: names …, which core.switches lacks`. Was ein
Leser schreibt, prüft er genauso, bevor es die Hand verlässt.

Die Prüfung verwendet den Teil von JSON Schema, den diese Schemas brauchen: `type`, `enum`, `const`, `properties`,
`required`, `additionalProperties`, `patternProperties`, `minProperties`, `items`, `minItems`, `maxItems`, `minimum`,
`maximum`, `minLength`, `pattern`, `oneOf`, `anyOf`, `allOf` und `$ref`, auch in ein anderes Schema über dessen `$id`
(z. B. `core#/$defs/TrackEnd`).

Darüber hinaus prüfen die Werkzeuge den Inhalt fachlich: ob Orte auf ihrem Gleis liegen, ob Signale nur Begriffe
zeigen, die der Katalog erlaubt, ob Fahrstraßen zu ihren Signalen passen. Solche Befunde schreibt das Format nicht
vor; ein Projektserver checkt keinen Stand ein, der neue Befunde dieser Art bringt.

## Versionen

**Ein Schema ist eine Version.** Jede Änderung eines Schemas — auch eine, die nur eine Regel ändert, die das Schema
benennt — erhöht die Version seiner Domäne: die Nebenversion für Ergänzungen, die ein älterer Leser übergehen darf,
sonst die Hauptversion. Ein Leser bringt eine ältere Datei Schritt für Schritt auf seinen Stand, von Nummer zu
Nummer; ältere Stände werden einmal umgestellt, nicht dauerhaft nachsichtig gelesen.

Die Versionen der Domänen setzen die Zählung der Suite fort: core 4.0, civil 3.0, signalling 4.0, electrification
3.0; vehicles 1.0 und timetables 1.0 sind die bisherige Domäne operations 1.0, in zwei geteilt.

# Ein Beispiel

Eine Datei mit einem Gleis aus einer Geraden und einem Bogen, neu ab dem 1. Mai 2027, einer Betrachtungsgrenze an
seinem Ende, seiner Kilometrierungslinie, einem Mast mit einem Aufhängepunkt, der einen Fahrdraht mit 15 kV hält,
einem Fahrzeug und einem Zug von der Grenze nach Halle-Trotha, wo er auf diesem Gleis hält:

```json
{
  "format": "rail-across", "version": "1.0",
  "projectId": "5dbd8fed-f8dc-4995-8934-9d71be69cae0",
  "generator": { "name": "Rail-Across-Suite", "version": "0.9.0" },
  "savedAt": "2026-11-13T09:30:00Z",
  "infraData": {
    "format": "infra-data", "version": "1.0",
    "domains": {
      "core": {
        "version": "4.0", "schema": "sha256:1111…",
        "fields": { "id": "5dbd8fed-f8dc-4995-8934-9d71be69cae0", "title": "Halle-Trotha" },
        "collections": {
          "tracks": [ {
            "id": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b", "name": "6050.001",
            "trackType": "line", "lineNumber": 6050, "epsg": 5684, "heightEpsg": 7837,
            "lifespan": { "from": "2027-05-01" },
            "elements": [
              { "elementType": "straight", "bearing": 90, "length": 100, "designSpeed": 100,
                "startNode": [4497120.512, 5706210.004],
                "endNode": [4497220.512, 5706210.004] },
              { "elementType": "arc", "bearing": 90, "length": 100, "radius": 600,
                "cant": 60, "designSpeed": 100,
                "startNode": [4497220.512, 5706210.004],
                "endNode": [4497320.050, 5706201.690] }
            ],
            "heights": [ { "station": 0, "z": 108.55 }, { "station": 200, "z": 109.15 } ]
          } ],
          "switches": [],
          "endMarks": [ {
            "id": "b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e", "kind": "boundary",
            "name": "Grenze Ost", "speed": 100,
            "at": { "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b", "endpoint": "END" }
          } ],
          "kmLines": [ {
            "id": "0f9e8d7c-6b5a-4f3e-8d2c-1b0a9f8e7d6c", "lineNumber": 6050, "epsg": 5684,
            "runs": [ { "kmStart": 12400.0, "counting": "up", "elements": [
              { "elementType": "straight", "bearing": 90, "length": 250,
                "startNode": [4497050.0, 5706220.0],
                "endNode": [4497300.0, 5706220.0] } ] } ]
          } ],
          "operationalPoints": [ {
            "id": "b64cfd92-f881-43ad-82d7-f0d78ef78747", "name": "Halle-Trotha",
            "rl100": "LHT", "uic": "8023111"
          } ],
          "stoppingPlaces": []
        }
      },
      "electrification": {
        "version": "3.0", "schema": "sha256:2222…", "dependsOn": ["core"], "fields": {},
        "collections": {
          "contactWires": [ {
            "id": "c1d2e3f4-a5b6-4c7d-8e9f-0a1b2c3d4e5f", "name": "Kettenwerk 1",
            "voltage": "15000V"
          } ],
          "masts": [ {
            "id": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a", "name": "12/4",
            "kind": "cantilever",
            "at": { "anchor": "station", "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                    "station": 40, "side": "right", "offset": 3.2 }
          } ],
          "suspensionPoints": [ {
            "id": "e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a5b",
            "mastId": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a",
            "at": { "anchor": "station", "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                    "station": 40 },
            "contactWireId": "c1d2e3f4-a5b6-4c7d-8e9f-0a1b2c3d4e5f",
            "height": 5.5, "stagger": 0.3
          } ],
          "sectionInsulators": []
        }
      }
    }
  },
  "operations": {
    "format": "operations", "version": "1.0",
    "domains": {
      "vehicles": {
        "version": "1.0", "schema": "sha256:3333…", "fields": {},
        "collections": { "vehicles": [ {
          "id": "a7b8c9d0-e1f2-4a3b-8c4d-5e6f7a8b9c0d", "key": "osrd:br1440",
          "rollingStock": {
            "railjson_version": "3.3", "name": "BR 1440", "mass": 84000, "max_speed": 44.44,
            "effort_curves": {
              "default_mode": "15000V",
              "modes": { "15000V": { "is_electric": true, "curves": [ {
                "cond": { "comfort": "STANDARD" },
                "curve": { "speeds": [0, 20, 44.44],
                           "max_efforts": [300000, 300000, 135000] }
              } ] } }
            }
          }
        } ] }
      },
      "timetables": {
        "version": "1.0", "schema": "sha256:5555…", "dependsOn": ["vehicles"], "fields": {},
        "collections": {
          "timetables": [ {
            "id": "f0e1d2c3-b4a5-4968-8776-655443322110", "name": "Grundtakt"
          } ],
          "trains": [ {
            "id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
            "timetableId": "f0e1d2c3-b4a5-4968-8776-655443322110", "number": "16001",
            "formationId": "8b7c6d5e-4f3a-4b2c-9d1e-0f9a8b7c6d5e",
            "departure": 28800, "margin": "5 %", "distribution": "mareco",
            "points": [
              { "id": "7c6d5e4f-3a2b-4c1d-8e0f-9a8b7c6d5e4f",
                "endMarkId": "b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e", "stop": false },
              { "id": "6d5e4f3a-2b1c-4d0e-9f8a-7b6c5d4e3f2a",
                "operationalPointId": "b64cfd92-f881-43ad-82d7-f0d78ef78747",
                "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                "stop": true, "dwell": 60 }
            ]
          } ],
          "trainKinds": [],
          "formations": [ {
            "id": "8b7c6d5e-4f3a-4b2c-9d1e-0f9a8b7c6d5e", "name": "Lok mit 5 Wagen",
            "tractionUnits": [ { "vehicleId": "a7b8c9d0-e1f2-4a3b-8c4d-5e6f7a8b9c0d" } ]
          } ],
          "connections": [], "turnarounds": [], "delayScenarios": [], "studies": []
        }
      }
    }
  },
  "blobs": {}
}
```

Die Prüfsummen sind hier gekürzt; in einer Datei hat jede ihre 64 Hexziffern.

# Verhältnis zur heutigen Implementierung

Diese Spezifikation ist das Ziel des Datenmodells. Mit Stand 13. November 2026 weicht die Implementierung der Suite
in diesen Punkten ab; jeder wird in einem eigenen Schritt angeglichen (Entscheidung 0028 und die folgenden):

- Zwei Dateien statt einer: die Infrastruktur als infra-data 2.0 (`.railacross.json`, Domänen core 4.0, civil 3.0,
  signalling 4.0, electrification 2.1), der Betrieb als operations 1.0 (`.operations.json`) mit einer Domäne
  `operations`, in der die Fahrzeuge mitgeführt werden; eine Datei von vorher führt noch die Schemas mit.
- Ein Fahrzeug hält sein Rollmaterial als Blob und nennt seine Traktionsarten ein zweites Mal (`tractionModes`).
- Der Record, mit dem die Werkzeuge arbeiten, behält das Vokabular von vorher — die Arten als Zahlen,
  `operationalPoint`, `dep`, eine `turnout` mit `B1` als Zweiggleis —, Kilometrierungslinien in WGS 84 mit dem
  Kilometer je Punkt und Kennungen jeder Form; die Tür wandelt ihn ins Dokument und zurück (Entscheidung 0029,
  Phase 6 nimmt das heraus). Eine Kilometrierungslinie von vorher steht als Punkte da, noch nicht mit
  Trassierungselementen angenähert.
- Der Layer electrification hält elektrifizierte Gleisabschnitte mit ihrem System, noch keine Masten, Aufhängepunkte,
  Streckentrennungen und Fahrdrähte.
- Die Punkte eines Zugs nennen ihren Ort in einem Feld (`place`) und behalten den Namen, unter dem er gewählt wurde.
- Eine Domäne, die der Leser nicht kennt, wird abgelehnt, nicht erhalten.
