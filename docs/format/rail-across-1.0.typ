// rail-across-1.0 — das Projektformat für Eisenbahninfrastruktur und ihren Betrieb
// Stand: 13.11.2026. Der Anhang ist aus den Schemas erzeugt (docs/format/anhang.mjs).

#set document(title: "rail-across-1.0 — Formatbeschreibung")
#set page(paper: "a4", margin: (x: 2cm, y: 2.3cm), numbering: "1 / 1",
  header: context { if counter(page).get().first() > 1 [
    #set text(8pt, fill: gray.darken(30%))
    rail-across-1.0 — Formatbeschreibung #h(1fr) Stand 13.11.2026
  ] })
#set text(lang: "de", font: ("Libertinus Serif", "New Computer Modern"), size: 10.5pt)
#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => { v(0.8em); it; v(0.3em) }
#show raw.where(block: false): box.with(fill: luma(242), inset: (x: 2pt), outset: (y: 2pt), radius: 2pt)
#show raw.where(block: true): block.with(fill: luma(245), inset: 8pt, radius: 3pt, width: 100%)
#set table(stroke: 0.5pt + luma(180), inset: 4.5pt)
#show table: set par(justify: false)
#show table.cell.where(y: 0): set text(weight: "bold")
#show link: set text(fill: rgb("#1f4e9c"))

#let tbl(cols, ..rows) = table(columns: cols, fill: (_, y) => if y == 0 { luma(232) }, ..rows)

#align(center)[
  #text(22pt, weight: "bold")[rail-across-1.0]
  #v(-0.4em)
  #text(13pt)[Ein Austauschformat für Eisenbahninfrastruktur und ihren Betrieb — Formatbeschreibung]
  #v(0.2em)
  #text(9pt, fill: gray.darken(30%))[Stand 13. November 2026 · Anhang aus den Schemas erzeugt]
]

#v(0.6em)
#outline(depth: 2, indent: 1.2em)

= Zweck

rail-across-1.0 beschreibt ein Eisenbahnprojekt in *zwei* JSON-Dateien: die Infrastruktur und ihren Betrieb.

- *infra-data* hält den Planungsstand der Infrastruktur: das Gleisnetz mit seiner Trassierung und seinen
  Höhen, Weichen und Gleisenden, Bezugsdaten wie Kilometrierungslinien und Betriebsstellen, die Halteplätze —
  und darauf die Layer der Fachgebiete: Bahnsteige und Ingenieurbauwerke (`civil`), die Leit- und
  Sicherungstechnik (`signalling`), die Fahrleitung (`electrification`).
- *operations* hält den Betrieb: Fahrpläne mit ihren Zügen, Zuggattungen, Zugbildungen und die Fahrzeuge, mit
  denen sie fahren, Anschlüsse und Wenden, Verspätungsszenarien und Kapazitätsuntersuchungen.

Beide Dateien teilen eine Hülle und dieselben Grundregeln. Jede dient zum Austausch, zum Speichern und zum
Versionieren. Die Infrastruktur kann in Varianten geplant werden; der Betrieb gehört zu keiner Variante und
nennt die Infrastruktur nur über Kennungen. So lässt sich ein Fahrplan auf jede Variante legen.

Das Format folgt sechs Grundsätzen:

- *Entwurfswerte statt Polylinien.* Ein Gleis wird als das gespeichert, was entworfen wurde: Geraden,
  Kreisbögen und Übergangsbögen mit Länge, Radius und Überhöhung. Jede gezeichnete Koordinate wird daraus
  abgeleitet.
- *Ebene Koordinaten, eine Ebene je Gleis.* Trassierungen werden in projizierten Ebenen entworfen und vermessen
  (DB_REF/Gauß-Krüger, ETRS89/UTM, historische Lagesysteme). Ein Gleis behält die Ebene, in der es entworfen
  wurde. Wechselt die Ebene, treffen sich zwei Gleise an einem Verbinder.
- *Abgeleitetes wird nicht gespeichert.* Stationen, Polylinien, Weichensymbole, Bahnsteigflächen, die
  Ausdehnung von Freimeldeabschnitten und die Bauzustände folgen aus dem Rest. Was nicht gespeichert ist, kann
  ihm nicht widersprechen.
- *Feste Kennungen, geprüfte Verweise.* Jedes Objekt, auf das etwas verweist, hat eine eigene Kennung.
  Verweise nennen nur diese Kennung, nie einen Namen oder eine Listenposition, und jeder Verweis muss ein
  Objekt finden. Das macht das Zusammenführen zweier Stände Objekt für Objekt und Feld für Feld möglich.
- *Ein Modell.* Was gespeichert wird, ist das, womit gearbeitet wird: Eine Betriebsstelle ist ein Objekt mit
  Kennung, kein wiederholter Name; ein Bauzustand ist ein Tag, kein eigenes Verzeichnis.
- *Versionen, die zählen.* Jede Änderung eines Schemas erhöht die Version seiner Domäne. Ein Leser bringt
  eine ältere Datei Schritt für Schritt auf seinen Stand.

= Aufbau einer Datei

== Die beiden Dateien

#tbl((auto, auto, 1fr),
  [Datei], [Endung], [Inhalt],
  [infra-data 2.0], [`.railacross.json`], [die Domäne `core` und die Layer `civil`, `signalling`, `electrification`],
  [operations 1.0], [`.operations.json`], [die Domäne `operations`],
)

Zusammen bilden sie rail-across-1.0. Die Hülle jeder Datei nennt ihr eigenes Format und ihre eigene Version;
die Domänen darin haben je ihre eigene Version (@versionen).

== Hülle

Eine Datei besteht aus der Hülle, den Domänen, den mitgeführten Schemas und den Blobs:

```json
{
  "format": "infra-data",
  "version": "2.0",
  "domains": {
    "core":            { "version": "3.1", "schema": "sha256:…", "fields": { … }, "collections": { … } },
    "civil":           { "version": "2.1", "schema": "sha256:…", "dependsOn": ["core"],
                         "fields": { … }, "collections": { … } },
    "signalling":      { "version": "3.1", "schema": "sha256:…", "dependsOn": ["core"],
                         "fields": { … }, "collections": { … } },
    "electrification": { "version": "2.1", "schema": "sha256:…", "dependsOn": ["core"],
                         "fields": { … }, "collections": { … } }
  },
  "schemas": { "sha256:…": { … } },
  "blobs":   { "sha256:…": { … } }
}
```

Die Fahrplandatei hat dieselbe Hülle mit `"format": "operations"`, `"version": "1.0"` und genau einer
Domäne, `operations`.

#tbl((auto, 1fr),
  [Teil], [Inhalt],
  [`format`, `version`], [`infra-data` und `2.0`, oder `operations` und `1.0`],
  [`domains`], [die Domänen unter ihrem Namen. In infra-data ist `core` Pflicht, die Layer hängen von ihr ab und fehlen, wo ein Projekt nichts in ihnen hat],
  [`schemas`], [die JSON-Schemas der Domänen, je unter ihrer Prüfsumme — in einer Datei, die weitergegeben wird],
  [`blobs`], [große Datenmengen (Messpunkte, Rollmaterial), je unter der Prüfsumme ihres Inhalts],
)

== Domänen und Layer

Eine *Domäne* ist ein Fachgebiet mit eigenem Schema und eigener Version. Sie hat eigene Felder (`fields`) und
Sammlungen (`collections`). Ein *Layer* ist eine Domäne, die auf dem Kern aufsetzt: Sie nennt `core` unter
`dependsOn` und verweist nur in sich selbst und in den Kern. Jeder Layer wird in seinem eigenen Werkzeug
bearbeitet.

#tbl((auto, 1fr),
  [Domäne], [Inhalt],
  [`core` 3.1], [das Netz: Gleise, Weichen und Verbinder, Gleisenden (Prellböcke, Betrachtungsgrenzen), Kilometrierungslinien, Betriebsstellen, Halteplätze, benannte Gleisfolgen. Die Grundlage aller anderen],
  [`civil` 2.1], [Bahnsteige und Ingenieurbauwerke (Eisenbahnüberführungen, Stützwände, Lärmschutzwände), in Punktwolken gemessene Achsen, Bestandsachsen, Schriftfeld, Projektbild],
  [`signalling` 3.1], [die Leit- und Sicherungstechnik nach PlanPro 1.10: Signale mit logischen Signalen, Gleisfreimeldung, Weichenelemente, Fahrstraßen, Durchrutschwege, Flankenschutz, Zugbeeinflussung, Bahnhofsgrenzen, Fahrwege],
  [`electrification` 2.1], [die Abschnitte unter Fahrleitung und ihr System],
  [`operations` 1.0], [der Betrieb (eigene Datei): Fahrpläne, Züge, Zuggattungen, Zugbildungen, Fahrzeuge, Anschlüsse, Wenden, Verspätungsszenarien, Untersuchungen],
)

== Sammlungen und Objekte

- Eine Sammlung ist eine Liste von Objekten mit `id`, *sortiert nach `id`*. Derselbe Stand ergibt so dieselben
  Bytes und dieselbe Prüfsumme.
- Folgen innerhalb eines Objekts (`elements`, `heights`, die Punkte eines Zugs) behalten ihre Reihenfolge, sie
  haben eine Bedeutung.
- Jedes Objekt erlaubt nur die Properties, die sein Schema nennt (`additionalProperties: false`).
- Kennungen sind üblicherweise UUIDs in Kleinbuchstaben.

= Grundregeln

== Schemas und Prüfsummen

Eine Domäne nennt ihr Schema über `schema`: den SHA-256 seines *kanonischen JSON* nach RFC 8785 (JSON
Canonicalization Scheme), geschrieben als `sha256:` und 64 Hexziffern. Ein Dokument einer Version muss genau
das Schema dieser Version nennen. Eine Datei, die weitergegeben wird, führt die Schemas unter diesen
Prüfsummen mit, damit ein Empfänger sie auch ohne Kenntnis des Formats prüfen kann. Jeder Eintrag in
`schemas` und `blobs` muss genau die Prüfsumme seines Inhalts tragen.

== Verweise

Ein Verweis ist die nackte Kennung des Ziels. Im Schema trägt das Property die Anmerkung `x-target`, die die
Sammlung nennt, in die es zeigt, z. B. `core.tracks` oder `signalling.routes`. Jeder solche Verweis muss ein
Objekt der genannten Sammlung finden; ein Dokument, dessen Verweis ins Leere zeigt, wird nicht angenommen
(@pruefung). Gleisenden werden als Paar aus Gleis und Ende genannt (`TrackEnd`: `trackId` und
`endpoint` `BEGIN` oder `END`).

Die Fahrplandatei verweist in sich selbst (ein Zug auf seinen Fahrplan, seine Gattung, seine Zugbildung) und
in die Infrastruktur (ein Punkt eines Zugs auf eine Betriebsstelle, ein Gleis, einen Fahrweg). Die Verweise in
sich selbst prüft sie beim Einlesen; die in die Infrastruktur werden geprüft, wo der Betrieb auf eine Variante
gelegt wird.

== Orte auf dem Gleis

Wo ein Objekt steht, ist *ein* Wert (`TrackLocation`) mit einem Anker:

- `anchor: "station"` — das Objekt folgt dem Gleis: `trackId`, `station` (m ab Gleisanfang), `side`
  (`left`/`right`, gesehen in Stationierungsrichtung), `offset` (m von der Achse), `direction` (`up` mit der
  Stationierung, `down` dagegen). Wird ein Gleis geteilt, verbunden oder umgedreht, werden diese Orte mitgeführt.
- `anchor: "plane"` — das Objekt behält seinen Platz in der Ebene: `easting`, `northing`, `epsg`, optional
  `bearing`. So bleibt ein Objekt erhalten, dessen Gleis entfernt wurde.

Ein Abschnitt eines Gleises ist ein `TrackRange` (`trackId`, `startStation`, `endStation`, `side`), eine
Gleisfolge ein `TrackPath` (`trackIds` in Fahrtrichtung; die Richtung je Gleis folgt daraus, wo sie
aneinanderstoßen).

== Lebensdauer und Bauzustände <lebensdauer>

Jedes Objekt kann eine *Lebensdauer* tragen: die Tage, an denen es in Betrieb ist.

#tbl((auto, 1fr),
  [Lebensdauer], [Bedeutung],
  [`{ "from": "2027-05-01" }`], [neu ab dem 1. Mai 2027],
  [`{ "from": "2027-05-01", "to": "2031-12-12" }`], [vorübergehend, vom 1. Mai 2027 bis zum 12. Dezember 2031],
  [`{ "from": "open" }`], [neu, der Tag steht noch nicht fest],
  [`{ "to": "open" }`], [wird zurückgebaut, der Tag steht noch nicht fest],
  [keine], [besteht und bleibt],
)

`from` ist der erste Tag in Betrieb, `to` der letzte. Ohne `from` besteht das Objekt heute, ohne `to` bleibt
es. Ein Objekt, das weder neu gebaut noch zurückgebaut wird, trägt *keine* Lebensdauer. `open` steht, wo der
Tag noch nicht feststeht.

Die *Bauzustände* werden nicht gespeichert, sie folgen aus den Lebensdauern: der Bestand, ein Zustand für
jeden Tag, an dem etwas in oder außer Betrieb geht, und der Endzustand.

#tbl((auto, auto, auto, auto),
  [Lebensdauer], [im Bestand], [an einem Tag $T$], [im Endzustand],
  [keine], [ja], [ja], [ja],
  [`from`: Tag $F$], [nein], [wenn $F <= T$], [ja],
  [`from`: `open`], [nein], [nein], [ja],
  [`to`: Tag $L$], [ja], [wenn $T <= L$], [nein],
  [`to`: `open`], [ja], [ja], [nein],
)

Bei beiden Angaben müssen beide Bedingungen gelten. Der Anzeigestatus folgt ebenso: mit `from` *neu*, mit
`to` allein *Rückbau*, sonst *Bestand*. Eine Weiche ohne eigene Lebensdauer zeigt, was ihre Gleise sagen.

== Blobs

Große Datenmengen stehen nicht im Objekt, sondern in `blobs`: die Punkte gemessener Achsen und Bestandsachsen
(`civil`) und das Rollmaterial der Fahrzeuge (`operations`). Das Objekt nennt den Blob über die Prüfsumme
seines kanonischen JSON. Gleiche Inhalte stehen so nur einmal in der Datei.

= Das Netz (Domäne `core`)

== Gleise und Trassierungselemente

Ein Gleis (`tracks`) hat eine Ebene (`epsg`), in der alle seine Elemente angegeben sind, ein
Höhenbezugssystem (`heightEpsg`), die Folge seiner Trassierungselemente (`elements`) und seine Höhen
(`heights`). Ein Bahnhofsgleis nennt seine Betriebsstelle (`operationalPoint`) und seine Gleisnummer, ein
Streckengleis seine Strecke.

*Ein Element hat keine Station und keine Polylinie.* Die Station eines Punkts ist die laufende Summe der
Längen vom Gleisanfang. Die Geometrie folgt aus dem Anfangspunkt, der Richtung und den Entwurfswerten.

#tbl((auto, 1fr, 1fr, 1fr),
  [Wert], [Gerade `0`], [Kreisbogen `1`], [Übergangsbogen `2`],
  [`startNode`, `endNode`], [`[E, N]` in m], [`[E, N]` in m], [`[E, N]` in m],
  [`bearing`], [Richtung am Anfang, Grad, von Gitternord im Uhrzeigersinn], [ebenso], [ebenso],
  [`length`], [m], [m, Bogenlänge], [m],
  [`radius`], [—], [m, vorzeichenbehaftet: + Rechtsbogen], [—],
  [`r1`, `r2`], [—], [—], [Radien am Anfang und Ende, vorzeichenbehaftet; `null` = gerades Ende],
  [`transitionType`], [—], [—], [`clothoid` oder `bloss`],
  [`cant`], [mm, an beiden Enden gleich], [mm], [—],
  [`cantStart`, `cantEnd`], [mm, wo die Überhöhung entlang des Elements wechselt], [ebenso], [mm, immer beide],
  [`endBearing`], [nur bei einem Knick], [nur wenn abweichend], [nur wenn abweichend],
)

Ein Element gibt seine Überhöhung als `cant` *oder* als `cantStart` und `cantEnd` an, nie beides. Eine
fehlende Überhöhung ist 0. Die Überhöhung ist vorzeichenbehaftet nach der überhöhten Schiene, in
Stationierungsrichtung gesehen. Dazu kommen die Entwurfsgeschwindigkeit (`designSpeed`, km/h), die Rolle bei
Änderungen (`role`), Begründungen zu Regelhinweisen (`reasons`) und an Weichenstrecken die Markierung
`switchBranch`, `switchRoute`, `switchId`. Ein Element, das als Weichenstrecke markiert ist, nennt seine Weiche.

== Höhen

Die Höhen sind ein Neigungsbruchpolygon, unabhängig von den Trassierungselementen: Punkte mit `station` (m ab
Gleisanfang) und `z` (m, Oberkante der nicht überhöhten Schiene), optional mit Ausrundungsradius `rv` und der
Länge `la`, die ihn bestimmt. Ein Punkt kann von einem anderen abhängen (`dep`, `depMode`), etwa in einer
Weiche. Zwischen zwei Punkten ist die Neigung konstant.

== Weichen und Verbinder

Ein Objekt der Sammlung `switches` sitzt an einem Knoten. Seine *Ports* halten Gleisenden (`TrackEnd`). Welche
Ports es hat, legt `kind` fest:

#tbl((auto, auto, 1fr),
  [`kind`], [Ports], [Bedeutung],
  [`turnout`], [`A`, `B1`, `B2`], [Weiche: `A` Weichenanfang, `B2` Stammgleis, `B1` Zweiggleis],
  [`crossing`], [`A`, `B`, `C`, `D`], [Kreuzung],
  [`single_slip`, `double_slip`], [`A`, `B`, `C`, `D`], [einfache und doppelte Kreuzungsweiche],
  [`link`], [`A`, `B`], [Verbinder: zwei Gleisenden, wo die Ebene wechselt oder ein umgebautes Stück anschließt],
  [`stage_link`], [`A`, `B1`, `B2`], [Bauzustandsverbinder: das Gleis an `A` ist mit dem an `B1` verbunden, solange es besteht, danach mit dem an `B2`],
)

Die Strecken einer Weiche sind die Elemente der angeschlossenen Gleise, die mit `switchId` und `switchRoute`
markiert sind. Die Form steht in `label` (z. B. `300 – 1:9`) und verweist auf einen Formkatalog, `formVersion`
nennt dessen Version. Das Symbol der Weiche wird nicht gespeichert.

== Betriebsstellen und Halteplätze

Eine *Betriebsstelle* (`operationalPoints`) ist ein Objekt mit fester Kennung: Bahnhof, Haltepunkt oder eine
andere Betriebsstelle, mit Name, RL100-Kürzel und UIC-Nummer. Gleise, Bahnsteige, Signale, Halteplätze und die
Punkte der Züge verweisen auf sie; ihr Name steht nur bei ihr. Umbenennen ändert genau ein Feld. Ihre Grenzen
legt die Leit- und Sicherungstechnik fest (`stationBoundaries`).

Ein *Halteplatz* (`stoppingPlaces`) ist, wo Züge halten: ein Abschnitt eines Gleises, im Bahnhof oder auf der
freien Strecke, für jeden Zug. Er gehört zum Kern, weil Leit- und Sicherungstechnik, Bahnsteige und Fahrpläne
ihn gleichermaßen brauchen. Ein *Personenhalt* (`passengerStop`) ist er, wo ein Bahnsteig an ihm liegt und ihn
nennt (Layer `civil`, `stoppingPlaceIds`).

== Weitere Sammlungen

- `endMarks`: Prellbock (`buffer_stop`) oder Betrachtungsgrenze (`boundary`) an einem freien Gleisende. Eine
  Grenze nennt die Geschwindigkeit, mit der Züge sie überqueren, und den kleinsten Abstand zweier Züge.
- `kmLines`: Kilometrierungslinien als fremde Bezugsdaten in WGS 84; der dritte Wert jedes Punkts ist der
  Kilometer in m. Sie gelten für Gleise in verschiedenen Ebenen. Ihre Kennung bilden die Quellen aus der
  Streckennummer (`km-6050`).
- `trackPaths`: benannte Gleisfolgen, für Längsschnitte und Tabellen der Trassierung.

= Bahnsteige und Bauwerke (Layer `civil`)

Der Layer hält, was im Gleisnetz steht, ohne zum Netz zu gehören:

- *Bahnsteige* (`platforms`) als `TrackRange` mit Kantenabstand, Höhe und Betriebsstelle. Ein Bahnsteig nennt die
  Halteplätze an ihm (`stoppingPlaceIds`), die dadurch Personenhalte sind.
- *Ingenieurbauwerke*: Eisenbahnüberführungen (`bridges`) mit Tragwerk, Feldern und lichter Höhe, Stützwände
  (`retainingWalls`) und Lärmschutzwände (`noiseBarriers`) mit Bauart, Abstand und Höhe — jeweils als Abschnitt
  eines Gleises. Wird das Gleis geteilt oder umgedreht, werden sie mitgeführt.
- *Gemessene Achsen* (`axisSurveys`, Punkte als ganze Millimeter in parallelen Listen, als Blob) und
  *Bestandsachsen* aus Vermessungen (`referenceAxes`, ein Punkt je Zentimeter, als Blob).
- Das Schriftfeld des Plans und das Projektbild (`fields`).

= Die Leit- und Sicherungstechnik (Layer `signalling`)

Der Layer folgt PlanPro 1.10. Jeder Standort steht auf einem Gleis des Kerns.

- *Signale* (`signals`) sind physische Standorte, real oder fiktiv. Ein reales Signal trägt je Signalsystem ein
  *logisches Signal* (z. B. Ks-Hauptsignal, Sh-Signal) mit Schirm, Zusatzanzeigern und der signalisierten
  Geschwindigkeit je Fahrstraße. Tafeln ohne System (Ne, Lf, Ra, Zs …) sind Rahmen des Standorts selbst. Ein
  fiktives Signal markiert, wo eine Fahrstraße ohne Signal beginnt oder endet, etwa an einer Betrachtungsgrenze.
- *Begriffe* nennen ihren Schlüssel im Katalog, den PlanPro-Namen (`Hp_0`, `Ks_1`, `Zs_3`, `Lf_7`), mit
  Symbol, wo der Katalog eines verlangt: bei Zs 3 und Lf 7 die Geschwindigkeit in Zehnern km/h (`7` = 70 km/h).
- *Gleisfreimeldung*: Freimeldepunkte (`detectionPoints`) begrenzen Abschnitte (`trackSections`). Die
  Ausdehnung eines Abschnitts wird nicht gespeichert, sie folgt aus seinen Punkten über das Netz.
- *Weichenelemente* (`switchElements`) sind die Weichen des Kerns und Gleissperren, wie das Stellwerk sie kennt.
- *Fahrstraßen* (`routes`) führen von einem Start- zu einem Zielsignal über einen `TrackPath`, mit
  Fahrstraßenbilde- und Auflösezeit, wo sie von denen des Layers abweichen. Die Weichenlagen folgen aus der
  Gleisfolge. Dazu kommen *Durchrutschwege* (`overlaps`) und *Flankenschutz* (`flankProtection`).
- *Zugbeeinflussung*: punktförmig (`balises`: PZB-Magnete, Geschwindigkeitsprüfungen, ETCS-Balisen) und über
  Gleisabschnitte (`trainProtection`: PZB, LZB, ETCS mit Level).
- *Bahnhofsgrenzen* (`stationBoundaries`) und *Fahrwege* (`itineraries`) von Halteplatz zu Halteplatz.

Welche Signalsysteme, Signalarten, Begriffe und Einstellungen es gibt, legt nicht das Schema fest, sondern ein
*Katalog* nach DB Ril 301. Der Layer nennt ihn in `fields.catalogue` (`id`, `version`). Ein neues
Signalsystem ist ein neuer Katalogeintrag, kein neues Schema.

= Die Fahrleitung (Layer `electrification`)

Ein Eintrag (`electrifications`) ist ein Abschnitt eines Gleises unter Fahrleitung eines Systems, genannt als
Spannung, wie OSRD die Traktionsart nennt (`15000V`, `25000V`, `3000V`, `1500V`, `750V`, `600V`). Ein Zug fährt
dort elektrisch, wo sein Fahrzeug dieses System hat.

= Der Betrieb (Datei operations)

Die Fahrplandatei hält den ganzen Betrieb eines Projekts. Sie gehört zu keiner Variante der Infrastruktur.

- Ein *Fahrplan* (`timetables`) sammelt Züge und sagt, was geschieht, wo sie sich begegnen (`dispatching`:
  nur Konflikte zeigen oder die Züge nacheinander ins Netz geben, nach Abfahrt oder nach Gattung), und welche
  Teile der Sperrzeit der Betrieb vorgibt (`blockingTimes`).
- Ein *Zug* (`trains`) hat eine Nummer, eine Gattung, eine Zugbildung, eine Abfahrt in Sekunden ab Mitternacht
  des Fahrplantags (über 24:00 der nächste Tag), einen Fahrzeitzuschlag und seine *Punkte*: Betriebsstellen
  oder Gleisenden (Prellbock, Betrachtungsgrenze) über ihre Kennung, mit Gleis, Halt, Haltezeit, Abfahrt und
  dem Fahrweg, den er zum nächsten Punkt bevorzugt. Jeder Punkt behält den Namen, unter dem der Ort gewählt
  wurde, damit eine Variante, der er fehlt, sagen kann, welcher gemeint war.
- *Zuggattungen* (`trainKinds`) mit Rang und Farbe; *Zugbildungen* (`formations`) aus Triebfahrzeugen,
  Wagenzuglast und Bremse.
- *Fahrzeuge* (`vehicles`): Jedes Fahrzeug, das eine Zugbildung nennt, führt die Datei mit, als
  RailJSON-Rollmaterial im Blob. Die Datei rechnet mit dem, was sie mitführt, nicht mit einem Katalog, der sich
  inzwischen geändert haben kann.
- *Anschlüsse* (`connections`) und *Wenden* (`turnarounds`) zwischen Zügen.
- *Verspätungsszenarien* (`delayScenarios`) und *Kapazitätsuntersuchungen* (`studies`) nach UIC 406.

= Einheiten und Genauigkeit

#tbl((auto, auto, 1fr),
  [Größe], [Einheit], [Genauigkeit],
  [Koordinaten, Längen, Radien, Stationen], [m], [0,1 µm (7 Nachkommastellen)],
  [Höhen `z`], [m], [0,1 mm (4 Nachkommastellen)],
  [Richtungswinkel], [°, von Gitternord im Uhrzeigersinn], [10#super[−9] °],
  [Überhöhung], [mm], [0,001 mm],
  [Geschwindigkeiten], [km/h, Fahrzeuge m/s], [wie angegeben],
  [Zeiten im Betrieb], [s], [wie angegeben],
  [Kilometrierung in `kmLines`], [m], [wie geliefert],
)

Positionen werden so fein gehalten, weil Werte beim erneuten Einlesen aneinandergereiht werden: Ein gerundeter
Punkt würde an einer Weiche einen Tangentenknick erzeugen, und ein gerundeter Richtungswinkel lässt ein langes
Gleis seitlich abwandern.

= Prüfung und Versionen <versionen>

== Prüfung beim Einlesen <pruefung>

Ein Leser nimmt eine Datei erst nach einer Prüfung an:

+ Format und Version der Hülle; anderes wird als `unsupported_version` abgelehnt.
+ Die Prüfsummen in `schemas` und `blobs`.
+ Die Version jeder Domäne. Eine ältere Version wird auf den Stand des Lesers gebracht (unten); eine neuere,
  als dieser Leser kennt, wird mit eben diesem Hinweis abgelehnt.
+ Die Prüfsumme des Schemas jeder Domäne.
+ Jede Domäne gegen ihr Schema, und jede Kennung nur einmal in ihrer Sammlung.
+ Was ein Schema nicht ausdrücken kann: Die Tage einer Lebensdauer sind Tage, der erste liegt nicht nach dem
  letzten; ein als Weichenstrecke markiertes Element nennt seine Weiche; *jeder Verweis findet sein Objekt*.

Fehler der Schritte 2, 4, 5 und 6 werden als `invalid_payload` mit den gefundenen Stellen gemeldet, z. B.
`/domains/signalling/collections/switchElements/0/switchId: names w9, which core.switches lacks`. Was ein Leser
schreibt, prüft er genauso, bevor es die Hand verlässt.

Die Prüfung verwendet den Teil von JSON Schema, den diese Schemas brauchen: `type`, `enum`, `const`,
`properties`, `required`, `additionalProperties`, `patternProperties`, `minProperties`, `items`, `minItems`,
`maxItems`, `minimum`, `maximum`, `minLength`, `pattern`, `oneOf`, `anyOf`, `allOf` und `$ref`, auch in ein
anderes Schema über dessen `$id` (z. B. `core#/$defs/TrackEnd`).

Darüber hinaus prüfen die Werkzeuge den Inhalt fachlich: ob Orte auf ihrem Gleis liegen, ob Signale nur
Begriffe zeigen, die der Katalog erlaubt, ob Fahrstraßen zu ihren Signalen passen. Solche Befunde schreibt das
Format nicht vor; der Projektserver checkt keinen Stand ein, der neue Fehler dieser Art bringt.

== Versionen

*Ein Schema ist eine Version.* Jede Änderung eines Schemas — auch eine, die nur eine Regel ändert, die das
Schema benennt — erhöht die Version seiner Domäne. Ein Leser bringt eine ältere Datei Schritt für Schritt auf
seinen Stand; ältere Stände werden einmal umgestellt, nicht dauerhaft nachsichtig gelesen.

#tbl((22%, 24%, 1fr),
  [Schritt], [Versionen], [was er tut],
  [infra-data 1.0 → 2.0], [Hülle 1.0 → 2.0], [`layout` und die Bahnsteige werden `civil`, die Halteplätze kommen aus der Leit- und Sicherungstechnik in den Kern; eine Bahnhofsgrenze an einem Bahnsteig wird an seinem Halteplatz markiert],
  [Lebensdauer nach Tagen], [core 2.0 → 3.0 \ civil 1.0 → 2.0 \ signalling 2.0 → 3.0 \ electrification 1.0 → 2.0], [das Verzeichnis der Bauzustände entfällt: eine Lebensdauer, die eine Stufe nannte, nennt ihren Tag (`to`: den Tag davor); der Endzustand wird `open`, der Bestand keine Angabe],
  [Verweise finden ihr Objekt], [core 3.0 → 3.1 \ civil 2.0 → 2.1 \ signalling 3.0 → 3.1 \ electrification 2.0 → 2.1], [ein Verweis, der ins Leere zeigt, wird entfernt, wie das Löschen seines Ziels ihn entfernt hätte: ein optionaler Verweis allein, ein Listeneintrag, sonst das Objekt, das ihn braucht — und so fort, bis nichts mehr ins Leere zeigt],
)

== Ausblick

Für die nächsten Versionen sind beschlossen (Entscheidung 0028 der Suite): Mitgeführte Schemas entfallen, die
Prüfsumme eines Schemas dient nur noch der Unversehrtheit; unbekannte Domänen und Felder werden durchgereicht
statt abgelehnt; die Hülle nennt das erzeugende Programm, den Speicherzeitpunkt und die Projekt-Kennung; das
Kernschema wird bereinigt (eine maßgebliche Angabe je Element, Wörter statt Zahlen, ein EPSG, einheitliche
Namen); Kilometrierungslinien werden Blobs; die Fahrplandatei nennt Projekt, Infrastrukturstand und Tag.

= Ein kleines Beispiel

Ein Gleis aus einer Geraden und einem Bogen, neu ab dem 1. Mai 2027, und an seinem Ende eine
Betrachtungsgrenze (gekürzt):

```json
{
  "id": "6e1f…", "name": "6050.001", "trackType": 1, "lineNumber": 6050,
  "epsg": 5684, "heightEpsg": 7837, "lifespan": { "from": "2027-05-01" },
  "elements": [
    { "elementType": 0, "bearing": 90, "length": 100, "designSpeed": 100,
      "startNode": [4497120.512, 5706210.004],
      "endNode": [4497220.512, 5706210.004] },
    { "elementType": 1, "bearing": 90, "length": 100, "radius": 600, "cant": 60,
      "designSpeed": 100,
      "startNode": [4497220.512, 5706210.004],
      "endNode": [4497320.050, 5706201.690] }
  ],
  "heights": [ { "station": 0, "z": 108.55 }, { "station": 200, "z": 109.15 } ]
}
```

```json
{ "id": "b2…", "kind": "boundary", "at": { "trackId": "6e1f…", "endpoint": "END" },
  "name": "Grenze Ost", "speed": 100, "lifespan": { "from": "2027-05-01" } }
```

Ein Zug der Fahrplandatei, von der Grenze nach Halle-Trotha, wo er auf Gleis 1 hält:

```json
{ "id": "z-1", "timetableId": "tt-1", "number": "16001",
  "trainKindId": "rb…", "formationId": "lint41…",
  "departure": 28800, "margin": "5 %", "distribution": "mareco",
  "points": [
    { "id": "p1", "place": "b2…", "station": "Grenze Ost", "stop": false, "dwell": 0 },
    { "id": "p2", "place": "b64c…", "station": "Halle-Trotha", "track": "t2…",
      "stop": true, "dwell": 60 }
  ] }
```

#pagebreak()
#include "rail-across-1.0-anhang.typ"
