// Aus den Schemas erzeugt: node docs/format/anhang.mjs — nicht von Hand ändern.

= Anhang: alle Properties und ihre Werte <anhang>

Der Anhang ist aus den Schemas erzeugt (`infra-data.schema.json`, `core/3.1.schema.json`, `civil/2.1.schema.json`, `signalling/3.1.schema.json`, `electrification/2.1.schema.json`, `operations.schema.json`, `operations/1.0.schema.json`). Für jeden Objekttyp nennt er jedes Property mit seinem Pfad (verschachtelte Objekte mit Punkt, Listeneinträge mit `[]`), ob es Pflicht ist, seinen Typ mit allen zulässigen Werten und seine Bedeutung. Ein Verweis auf einen Typ führt zu dessen Abschnitt; `→ core.tracks` nennt die Sammlung, in die eine Kennung zeigt — die Prüfung beim Einlesen stellt sicher, dass es das Objekt gibt.

== Hülle von infra-data 2.0

=== Datei <infra-data-root>

Die Datei: Format, Version, Domänen, mitgeführte Schemas und Blobs.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`format`], [ja], [genau `infra-data`], [Kennung des Formats],
  [`version`], [ja], [genau `2.0`], [Version der Hülle],
  [`domains`], [ja], [Objekt; Schlüssel nach `^[a-z][a-zA-Z0-9]*$`: #link(<infra-data-Domain>)[`Domain`]], [Die Domänen, je unter ihrem Namen (Kleinbuchstabe am Anfang, dann Buchstaben und Ziffern); `core` ist Pflicht],
  [`schemas`], [], [Objekt; Schlüssel nach `^sha256:[0-9a-f]{64}$`: Objekt], [Die Schemas der Domänen, je unter der Prüfsumme, mit der die Domäne es nennt],
  [`blobs`], [ja], [Objekt; Schlüssel nach `^sha256:[0-9a-f]{64}$`: beliebiger Wert], [Große Listen, die Objekte über ihre Prüfsumme nennen; jeder Wert hat als kanonisches JSON genau diese Prüfsumme],
)

=== Domain <infra-data-Domain>

Eine Domäne in der Datei: ihre Version, ihr Schema, ihre Felder und Sammlungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`version`], [ja], [Text; Muster `^[0-9]+\.[0-9]+$`], [Version der Domäne (Haupt- und Nebenversion)],
  [`schema`], [ja], [Text; Muster `^sha256:[0-9a-f]{64}$`], [Prüfsumme (SHA-256 des kanonischen JSON) des Schemas, gegen das die Domäne geschrieben ist],
  [`dependsOn`], [], [Liste von Text], [Domänen, auf die sie verweist (für die Layer: `core`)],
  [`fields`], [ja], [Objekt], [Die eigenen Felder der Domäne (Aufbau im Schema der Domäne)],
  [`collections`], [ja], [Objekt; Schlüssel frei, Werte: Liste von Objekt], [Die Sammlungen: je eine Liste von Objekten mit `id`, sortiert nach `id`],
)

== Domäne core 3.1

=== Domänendokument <core-root>

Die Domäne core: die Felder des Projekts und die Sammlungen des Netzes.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`fields`], [ja], [#link(<core-Fields>)[`Fields`]], [Die Felder der Domäne],
  [`collections`], [ja], [Objekt (Properties unten)], [Die Sammlungen der Domäne],
  [`collections.tracks`], [ja], [Liste von #link(<core-Track>)[`Track`]], [Gleise],
  [`collections.switches`], [ja], [Liste von #link(<core-Switch>)[`Switch`]], [Weichen, Kreuzungen und Verbinder],
  [`collections.endMarks`], [ja], [Liste von #link(<core-EndMark>)[`EndMark`]], [Prellböcke und Betrachtungsgrenzen],
  [`collections.kmLines`], [ja], [Liste von #link(<core-KmLine>)[`KmLine`]], [Kilometrierungslinien],
  [`collections.operationalPoints`], [ja], [Liste von #link(<core-OperationalPoint>)[`OperationalPoint`]], [Betriebsstellen-Verzeichnis],
  [`collections.stoppingPlaces`], [ja], [Liste von #link(<core-StoppingPlace>)[`StoppingPlace`]], [Halteplätze],
  [`collections.trackPaths`], [], [Liste von #link(<core-NamedTrackPath>)[`NamedTrackPath`]], [benannte Gleisfolgen],
)

=== Id <core-Id>

Kennung eines Objekts, eindeutig in seiner Sammlung; üblich ist eine UUID in Kleinbuchstaben.

Wert: Text; mindestens 1 Zeichen

=== Ref <core-Ref>

Verweis auf ein Objekt über seine nackte Kennung. Wohin er zeigt, sagt `x-target` am Property; der Leser prüft, dass es das Objekt gibt.

Wert: Text; mindestens 1 Zeichen

=== Text <core-Text>

Freier Text.

Wert: Text

=== Label <core-Label>

Eine Nummer, wie Menschen sie schreiben: Strecken- oder Gleisnummer, auch mit Buchstaben.

Wert: Text oder Zahl

=== Epsg <core-Epsg>

EPSG-Code eines Koordinatenbezugssystems (Lage oder Höhe).

Wert: ganze Zahl; ≥ 1

=== Point <core-Point>

Ein Punkt `[Rechtswert, Hochwert]` in m, optional mit Höhe, in der Ebene des Objekts.

Wert: Liste von Zahl; mindestens 2 Einträge; höchstens 3 Einträge

=== Side <core-Side>

Seite, gesehen in Stationierungsrichtung.

Wert: eins von `left`, `right`: #linebreak() `left` links #linebreak() `right` rechts

=== Endpoint <core-Endpoint>

Ein Gleisende.

Wert: eins von `BEGIN`, `END`: #linebreak() `BEGIN` Gleisanfang #linebreak() `END` Gleisende

=== LineCategory <core-LineCategory>

Streckenkategorie.

Wert: eins von `main`, `secondary`, `s_bahn`: #linebreak() `main` Hauptbahn #linebreak() `secondary` Nebenbahn #linebreak() `s_bahn` S-Bahn

=== TrackEnd <core-TrackEnd>

Ein Ende eines Gleises: was ein Weichenport, ein Verbinder oder eine Endmarke hält.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`trackId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.tracks`], [das Gleis],
  [`endpoint`], [ja], [#link(<core-Endpoint>)[`Endpoint`]], [welches seiner Enden],
)

=== TrackLocation <core-TrackLocation>

Wo ein Objekt steht, als ein Wert. Der Anker sagt, ob es dem Gleis folgt (`station`) oder seinen Platz in der Ebene behält (`plane`).

*Variante 1* (`anchor` = `station`):

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`anchor`], [ja], [genau `station`], [Anker des Orts — `station` folgt dem Gleis],
  [`trackId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.tracks`], [das Gleis],
  [`station`], [ja], [Zahl], [Station ab Gleisanfang \[m\]],
  [`side`], [], [#link(<core-Side>)[`Side`]], [Seite, gesehen in Stationierungsrichtung],
  [`offset`], [], [Zahl; ≥ 0], [Abstand von der Gleisachse \[m\]],
  [`direction`], [], [eins der Werte unter Bedeutung], [Wirkrichtung #linebreak() `up` mit der Stationierung (zum Ende) #linebreak() `down` gegen die Stationierung],
)

*Variante 2* (`anchor` = `plane`):

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`anchor`], [ja], [genau `plane`], [Anker des Orts — `plane` behält seinen Platz in der Ebene],
  [`easting`], [ja], [Zahl], [Rechtswert \[m\]],
  [`northing`], [ja], [Zahl], [Hochwert \[m\]],
  [`epsg`], [ja], [#link(<core-Epsg>)[`Epsg`]], [Ebene des Punkts],
  [`bearing`], [], [Zahl], [Wirkrichtung als Richtungswinkel \[°, von Gitternord im Uhrzeigersinn\]],
)

=== TrackRange <core-TrackRange>

Ein Abschnitt eines Gleises von einer Station bis zu einer anderen, auf einer Seite.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`anchor`], [ja], [genau `station`], [folgt dem Gleis],
  [`trackId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.tracks`], [das Gleis],
  [`startStation`], [ja], [Zahl], [Anfang \[m ab Gleisanfang\]],
  [`endStation`], [ja], [Zahl], [Ende \[m ab Gleisanfang\]],
  [`side`], [], [#link(<core-Side>)[`Side`]], [Seite, gesehen in Stationierungsrichtung],
)

=== TrackPath <core-TrackPath>

Eine Folge von Gleisen in Fahrtrichtung. In welcher Richtung jedes befahren wird, folgt daraus, wo sie aneinanderstoßen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`trackIds`], [ja], [Liste von #link(<core-Ref>)[`Ref`] → `core.tracks`], [die Gleise in Fahrtrichtung],
)

=== Day <core-Day>

Ein Tag (ISO 8601, `JJJJ-MM-TT`).

Wert: Text; Muster `^[0-9]{4}-[0-9]{2}-[0-9]{2}$`

=== LifespanDay <core-LifespanDay>

Ein Tag, oder das Wort `open`, wo der Tag noch nicht feststeht.

Wert: eins von: #link(<core-Day>)[`Day`]\; genau `open`

=== Lifespan <core-Lifespan>

Die Tage, an denen ein Objekt in Betrieb ist: `from` der erste, `to` der letzte, `open`, wo der Tag noch nicht feststeht (neu gebaut, zurückgebaut). Ohne `from` besteht es heute, ohne `to` bleibt es; ohne Lebensdauer besteht es und bleibt. Ein `from`, das ein Tag ist, liegt nicht nach einem `to`, das einer ist.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`from`], [], [#link(<core-LifespanDay>)[`LifespanDay`]], [erster Tag in Betrieb, oder `open` (neu gebaut, Tag noch offen)],
  [`to`], [], [#link(<core-LifespanDay>)[`LifespanDay`]], [letzter Tag in Betrieb, oder `open` (zurückgebaut, Tag noch offen)],
)

=== Fields <core-Fields>

Die Felder des Projekts.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Projekts],
  [`title`], [], [#link(<core-Text>)[`Text`]], [Titel],
  [`description`], [], [#link(<core-Text>)[`Text`]], [Beschreibung],
  [`creator`], [], [#link(<core-Text>)[`Text`]], [Ersteller],
  [`createdAt`], [], [#link(<core-Text>)[`Text`]], [Erstellzeitpunkt (ISO 8601)],
  [`crsSystem`], [], [#link(<core-Text>)[`Text`]], [Koordinatenbezugssystem des Projekts],
  [`lineCategory`], [], [#link(<core-LineCategory>)[`LineCategory`]], [Streckenkategorie des Projekts],
  [`gaugeProfile`], [], [#link(<core-Text>)[`Text`]], [Lichtraumprofil],
)

=== Element <core-Element>

Ein Trassierungselement eines Gleises: Gerade (0), Kreisbogen (1) oder Übergangsbogen (2). Ohne Station und ohne Polylinie: Stationen sind die laufende Summe der Längen, die Geometrie wird abgeleitet.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`elementType`], [ja], [eins der Werte unter Bedeutung], [Art des Elements #linebreak() `0` Gerade #linebreak() `1` Kreisbogen #linebreak() `2` Übergangsbogen],
  [`epsg`], [], [#link(<core-Epsg>)[`Epsg`]], [Ebene, in der das Element angegeben ist, wo es sie nennt: die seines Gleises],
  [`startNode`], [ja], [#link(<core-Point>)[`Point`]], [Anfangspunkt in der Ebene des Gleises \[m\]],
  [`endNode`], [ja], [#link(<core-Point>)[`Point`]], [Endpunkt \[m\]],
  [`bearing`], [], [Zahl], [Richtungswinkel am Anfang \[°, von Gitternord im Uhrzeigersinn\]],
  [`endBearing`], [], [Zahl], [Richtungswinkel am Ende, nur wo er von dem abweicht, was die Entwurfswerte ergeben (Knick)],
  [`length`], [ja], [Zahl; ≥ 0], [Länge \[m\], beim Bogen die Bogenlänge],
  [`radius`], [], [Zahl oder null], [Radius des Kreisbogens \[m\], vorzeichenbehaftet (+ Rechtsbogen)],
  [`r1`], [], [Zahl oder null], [Radius am Anfang des Übergangsbogens \[m\], vorzeichenbehaftet; `null` = gerades Ende],
  [`r2`], [], [Zahl oder null], [Radius am Ende des Übergangsbogens \[m\]; `null` = gerades Ende],
  [`transitionType`], [], [eins der Werte unter Bedeutung], [Form des Übergangsbogens #linebreak() `clothoid` Klothoide #linebreak() `bloss` Bloss-Kurve],
  [`cant`], [], [Zahl], [Überhöhung \[mm\], an beiden Enden gleich; vorzeichenbehaftet nach der überhöhten Schiene],
  [`cantStart`], [], [Zahl], [Überhöhung am Anfang \[mm\], wo sie sich entlang des Elements ändert (dann kein `cant`)],
  [`cantEnd`], [], [Zahl], [Überhöhung am Ende \[mm\]],
  [`cantException`], [], [#link(<core-Text>)[`Text`]], [Begründung für 100–120 mm Überhöhung in einer Weiche],
  [`role`], [], [eins der Werte unter Bedeutung], [wie das Element bei Änderungen nachgeführt wird; fehlt es, ist es ein Festelement #linebreak() `follow` Folgeelement: Länge und Radius vorgegeben, alles dahinter folgt #linebreak() `intermediate` Zwischenelement: Radius vorgegeben, zwischen den Nachbarn neu berechnet],
  [`reasons`], [], [Objekt; Schlüssel frei, Werte: Text], [schriftliche Begründung je Regelhinweis, unter der Kennung der Regel],
  [`designSpeed`], [], [Zahl; ≥ 0 oder null], [Entwurfsgeschwindigkeit \[km/h\], für die das Element entworfen und gegen die es geprüft ist; `null` oder fehlend = unbekannt],
  [`switchBranch`], [], [Wahrheitswert], [gehört zu einer Weichenstrecke],
  [`switchRoute`], [], [eins der Werte unter Bedeutung], [welche Strecke der Weiche #linebreak() `main` Stammgleis #linebreak() `branch` Zweiggleis #linebreak() `cross` Kreuzungsstrang #linebreak() `slip1` erster Bogen einer Kreuzungsweiche #linebreak() `slip2` zweiter Bogen],
  [`switchId`], [], [#link(<core-Ref>)[`Ref`] → `core.switches`], [die Weiche],
  [`switchHint`], [], [#link(<core-Text>)[`Text`]], [warum ein Import die Weiche nicht bilden konnte: Form, Nummer, Grund],
)

=== HeightPoint <core-HeightPoint>

Ein Punkt des Neigungsbruchpolygons eines Gleises.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`station`], [ja], [Zahl], [Station \[m ab Gleisanfang\]],
  [`z`], [ja], [Zahl], [Höhe der nicht überhöhten Schiene \[m\]],
  [`rv`], [], [Zahl oder null], [Ausrundungsradius an diesem Punkt \[m\]],
  [`la`], [], [Zahl oder null], [Ausrundungslänge, die den Radius bestimmt \[m\]],
  [`reason`], [], [#link(<core-Text>)[`Text`]], [warum sich die Neigung in einer Weiche ändert],
  [`locked`], [], [Wahrheitswert], [Höhe und Station des Punkts bleiben fest],
  [`id`], [], [#link(<core-Id>)[`Id`]], [eigene Kennung des Punkts, sobald ein anderer von ihm abhängt],
  [`dep`], [], [#link(<core-Ref>)[`Ref`] → Kennung eines anderen Höhenpunkts], [Kennung des Punkts, von dem dieser abhängt],
  [`depMode`], [], [eins der Werte unter Bedeutung], [wie er abhängt #linebreak() `lds` Überhöhung der Quelle über den Abstand (ldS-Schwelle) #linebreak() `continueBefore` Neigung vor der Quelle fortgesetzt #linebreak() `continueAfter` Neigung hinter der Quelle fortgesetzt #linebreak() `wa` gleiche Höhe (ältere Dateien) #linebreak() `continue` Neigung von der abgewandten Seite (ältere Dateien)],
  [`free`], [], [Wahrheitswert], [WA oder ldS einer Weiche von ihrer Regel gelöst],
)

=== Superstructure <core-Superstructure>

Ein Oberbauabschnitt (Schiene oder Schwelle) über einen Stationsbereich.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`type`], [ja], [#link(<core-Text>)[`Text`]], [Bauart (z. B. Schienenprofil 60E2, Schwelle B90)],
  [`from`], [ja], [Zahl], [Anfang \[m ab Gleisanfang\]],
  [`to`], [ja], [Zahl], [Ende \[m ab Gleisanfang\]],
)

=== Track <core-Track>

Ein Gleis: seine Kennzeichen, seine Ebene, seine Trassierungselemente, Höhen und sein Oberbau.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`trackType`], [], [eins der Werte unter Bedeutung], [Gleisart #linebreak() `1` Streckengleis #linebreak() `2` Bahnhofsgleis],
  [`lineNumber`], [], [#link(<core-Label>)[`Label`]], [Streckennummer],
  [`lineName`], [], [#link(<core-Text>)[`Text`]], [Streckenname],
  [`side`], [], [ganze Zahl], [Lage eines Streckengleises: 1 Richtungsgleis, 2 Gegenrichtungsgleis],
  [`trackNumber`], [], [#link(<core-Label>)[`Label`]], [Gleisnummer im Bahnhof],
  [`operationalPoint`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [Bahnhof, zu dem das Gleis gehört],
  [`owner`], [], [#link(<core-Text>)[`Text`]], [Eigentümer],
  [`trackUse`], [], [genau `siding`], [Nebengleis; fehlt es, ist es ein Hauptgleis — `siding` Nebengleis],
  [`lineCategory`], [], [#link(<core-LineCategory>)[`LineCategory`]], [Streckenkategorie, wo sie von der des Projekts abweicht],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
  [`epsg`], [ja], [#link(<core-Epsg>)[`Epsg`]], [die eine Ebene, in der alle Elemente angegeben sind],
  [`heightEpsg`], [], [#link(<core-Epsg>)[`Epsg`]], [Höhenbezugssystem der Höhen (z. B. 7837 = DHHN2016)],
  [`elements`], [ja], [Liste von #link(<core-Element>)[`Element`]\; mindestens 1 Eintrag], [Trassierungselemente vom Anfang zum Ende],
  [`heights`], [], [Liste von #link(<core-HeightPoint>)[`HeightPoint`]], [Neigungsbruchpolygon],
  [`rails`], [], [Liste von #link(<core-Superstructure>)[`Superstructure`]], [Schienenprofile über Stationsbereiche],
  [`sleepers`], [], [Liste von #link(<core-Superstructure>)[`Superstructure`]], [Schwellen über Stationsbereiche],
  [`lagesystem`], [], [#link(<core-Text>)[`Text`]], [DB-Lagesystem, aus dem ein Import das Gleis übernommen hat (EA0, DB0, …)],
  [`proviAxis`], [], [#link(<core-Text>)[`Text`]], [ProVI-Achse, aus der ein Import das Gleis übernommen hat],
  [`osrd`], [], [Objekt], [was ein RailJSON-Import mitbrachte und das Modell nicht hält; beim Export zurückgegeben],
)

=== Switch <core-Switch>

Eine Weiche, Kreuzung, Kreuzungsweiche, ein Verbinder (`link`) oder ein Bauzustandsverbinder (`stage_link`) an einem Knoten. Die Ports halten Gleisenden. Ein Verbinder verbindet zwei Gleisenden, wo die Ebene wechselt oder ein umgebauter Gleisteil anschließt; ein Bauzustandsverbinder verbindet das Gleis an A mit dem an B1, solange es besteht, und danach mit dem an B2.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art; sie legt die Ports fest: Weiche A, B1, B2; Kreuzung und Kreuzungsweichen A, B, C, D; Verbinder A, B; Bauzustandsverbinder A, B1, B2 #linebreak() `turnout` Weiche #linebreak() `crossing` Kreuzung #linebreak() `single_slip` einfache Kreuzungsweiche #linebreak() `double_slip` doppelte Kreuzungsweiche #linebreak() `link` Verbinder zweier Gleisenden #linebreak() `stage_link` Bauzustandsverbinder: erst B1, dann B2],
  [`formVersion`], [ja], [ganze Zahl], [Version der Formtabellen, gegen die die Weiche gebaut ist],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`number`], [], [Zahl], [Nummer],
  [`label`], [], [#link(<core-Text>)[`Text`]], [Form aus dem Weichenkatalog (z. B. `300 – 1:9`)],
  [`speed`], [], [Zahl], [Geschwindigkeit im abzweigenden Strang \[km/h\]],
  [`trailing`], [], [Wahrheitswert], [die Weiche öffnet gegen die Stationierung des Stammgleises],
  [`swapped`], [], [Wahrheitswert], [Außenbogenweiche, deren Zweig gerade wurde: als Grundform mit vertauschten Strängen gebaut],
  [`toeLead`], [], [genau `branch`], [die eine Höhe am Weichenanfang kommt vom Zweig, nicht vom Stammgleis — `branch` Zweig führt],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
  [`mainRadius`], [], [Zahl oder null], [Radius des Stammgleises, wo die Gleise ihn nicht angeben \[m\]],
  [`mainRadiusEnd`], [], [Zahl oder null], [Radius des Stammgleises am Weichenende \[m\]],
  [`pad`], [], [beliebiger Wert], [Punktadresse aus einem MDB-Import],
  [`ports`], [ja], [Objekt; Schlüssel nach `^(A|B|B1|B2|C|D)$`: eins von: #link(<core-TrackEnd>)[`TrackEnd`]\; null], [die Gleisenden an den Ports],
)

=== EndMark <core-EndMark>

Ein Prellbock oder eine Betrachtungsgrenze an einem freien Gleisende. Ein Zug darf an beiden beginnen und enden.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art #linebreak() `buffer_stop` Prellbock #linebreak() `boundary` Betrachtungsgrenze],
  [`at`], [ja], [#link(<core-TrackEnd>)[`TrackEnd`]], [das freie Gleisende],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name des Orts für Fahrpläne],
  [`type`], [], [ganze Zahl oder Text], [Bauart des Prellbocks],
  [`brakeLength`], [], [Zahl], [Bremsweg des Prellbocks \[m\]],
  [`speed`], [], [Zahl; ≥ 0], [an einer Betrachtungsgrenze: Geschwindigkeit, mit der Züge sie ein- oder ausfahrend überqueren \[km/h\]; ohne sie, was die Strecke dort zulässt],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
  [`headway`], [], [Zahl; ≥ 0], [an einer Betrachtungsgrenze: kleinster Abstand \[s\] zweier Züge, die sie in derselben Richtung überqueren],
)

=== KmLine <core-KmLine>

Fremde Bezugsdaten: der Abschnitt einer DB-Strecke um das Projekt, in WGS 84; der dritte Wert jedes Punkts ist der Kilometer in m.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung, von den Quellen aus der Streckennummer gebildet (`km-6050`)],
  [`lineNumber`], [ja], [#link(<core-Label>)[`Label`]], [Streckennummer],
  [`crs`], [], [genau `EPSG:4326`], [Bezugssystem (WGS 84)],
  [`clip`], [], [Liste von Zahl; genau 4 Einträge], [Ausschnitt `[West, Süd, Ost, Nord]`],
  [`runs`], [ja], [Liste von Liste von Liste von Zahl; genau 3 Einträge], [Linienzüge aus Punkten `[Länge, Breite, Kilometer in m]`],
  [`source`], [], [#link(<core-Text>)[`Text`]], [Import, aus dem die Linie stammt (mdb, vermesn)],
)

=== OperationalPoint <core-OperationalPoint>

Eine Betriebsstelle des Verzeichnisses (Bahnhof, Haltepunkt, …): ein Objekt mit fester Kennung. Gleise, Bahnsteige, Signale und Halteplätze verweisen auf sie; ihr Name steht nur hier.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`rl100`], [], [#link(<core-Text>)[`Text`]], [RL100-Kürzel],
  [`uic`], [], [#link(<core-Label>)[`Label`]], [UIC-Nummer],
)

=== StoppingPlace <core-StoppingPlace>

Wo Züge halten (Halteplatz): ein Abschnitt eines Gleises, im Bahnhof oder an einem Bahnsteig, für jeden Zug. Ein Personenhalt ist er, wo ein Bahnsteig an ihm liegt, der ihn nennt (civil, `stoppingPlaceIds`).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`operationalPointId`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [Bahnhof, zu dem er gehört; ohne: freie Strecke],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt des Gleises],
  [`passengerStop`], [], [Wahrheitswert], [Personenhalt: hier steigen Reisende ein und aus, an einem Bahnsteig],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== NamedTrackPath <core-NamedTrackPath>

Eine benannte Gleisfolge, für Längsschnitte und Tabellen der Trassierung.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`path`], [ja], [#link(<core-TrackPath>)[`TrackPath`]], [die Gleisfolge],
)

== Layer civil 2.1

=== Layerdokument <civil-root>

Der Layer civil: Bahnsteige, Ingenieurbauwerke, Messachsen und Bestandsachsen, Schriftfeld und Projektbild.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`fields`], [ja], [#link(<civil-Fields>)[`Fields`]], [Die Felder des Layers],
  [`collections`], [ja], [Objekt (Properties unten)], [Die Sammlungen des Layers],
  [`collections.platforms`], [ja], [Liste von #link(<civil-Platform>)[`Platform`]], [Bahnsteige],
  [`collections.axisSurveys`], [ja], [Liste von #link(<civil-AxisSurvey>)[`AxisSurvey`]], [gemessene Achsen],
  [`collections.referenceAxes`], [ja], [Liste von #link(<civil-ReferenceAxis>)[`ReferenceAxis`]], [Bestandsachsen],
  [`collections.bridges`], [], [Liste von #link(<civil-Bridge>)[`Bridge`]], [Eisenbahnüberführungen],
  [`collections.retainingWalls`], [], [Liste von #link(<civil-RetainingWall>)[`RetainingWall`]], [Stützwände],
  [`collections.noiseBarriers`], [], [Liste von #link(<civil-NoiseBarrier>)[`NoiseBarrier`]], [Lärmschutzwände],
)

=== BlobRef <civil-BlobRef>

Verweis auf einen Blob: der SHA-256 seines kanonischen JSON.

Wert: Text; Muster `^sha256:[0-9a-f]{64}$`

=== Fields <civil-Fields>

Die Felder des Layers civil.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`planHeader`], [], [Objekt], [Schriftfeld des Plans],
  [`image`], [], [Text], [Projektbild als Data-URL],
  [`imageHash`], [], [Text], [Projektbild als Blob, über seinen SHA-256],
  [`osrd`], [], [Objekt], [RailJSON-Objekte, die nicht modelliert sind; beim Export zurückgegeben],
)

=== AxisSurvey <civil-AxisSurvey>

Eine in Punktwolken gemessene Gleisachse in einer eigenen Ebene. Ihre Punkte sind ganze Millimeter in parallelen Listen, als Blob.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`epsg`], [ja], [#link(<core-Epsg>)[`Epsg`]], [Ebene der Punkte],
  [`rail`], [], [Text], [gemessene Schiene],
  [`guide`], [], [Objekt (Properties unten)], [woran die Messung geführt wurde],
  [`guide.kind`], [ja], [eins der Werte unter Bedeutung], [an einem Gleis oder einer gezeichneten Linie #linebreak() `track` Gleis #linebreak() `line` gezeichnete Linie],
  [`guide.trackId`], [], [#link(<core-Ref>)[`Ref`] → `core.tracks`], [das führende Gleis],
  [`cloudNames`], [], [Liste von Text], [Namen der Punktwolken],
  [`cloudRefs`], [], [Liste von beliebigen Werten], [Verweise auf die Punktwolken],
  [`createdAt`], [], [#link(<core-Text>)[`Text`]], [Erstellzeitpunkt],
  [`step`], [], [Zahl], [Punktabstand \[m\]],
  [`gaps`], [], [Liste von Objekten (Properties unten)], [Lücken der Messung],
  [`gaps[].from`], [ja], [Zahl], [Anfang \[m\]],
  [`gaps[].to`], [ja], [Zahl], [Ende \[m\]],
  [`points`], [ja], [#link(<civil-BlobRef>)[`BlobRef`]], [die Punkte, als Blob],
)

=== ReferenceAxis <civil-ReferenceAxis>

Die Achse eines vermessenen Gleises (Bestandsachse), ein Punkt je Zentimeter, als Blob.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`epsg`], [ja], [#link(<core-Epsg>)[`Epsg`]], [Ebene der Punkte],
  [`heightEpsg`], [], [eins von: #link(<core-Epsg>)[`Epsg`]\; null], [Höhenbezugssystem],
  [`source`], [], [Objekt], [Herkunft (Datei, Import)],
  [`points`], [ja], [#link(<civil-BlobRef>)[`BlobRef`]], [die Punkte, als Blob],
)

=== Platform <civil-Platform>

Ein Bahnsteig an einem Gleis. Er nennt die Halteplätze an ihm, die dadurch Personenhalte sind.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt und Seite des Gleises],
  [`frontOffset`], [], [Zahl], [Abstand der Bahnsteigkante von der Gleisachse \[m\]],
  [`backOffset`], [], [Zahl], [Abstand der Bahnsteigrückseite von der Gleisachse \[m\]],
  [`height`], [], [Zahl], [Bahnsteighöhe \[mm über SO\]],
  [`operationalPoint`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [Bahnhof oder Haltepunkt des Bahnsteigs],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
  [`stoppingPlaceIds`], [], [Liste von #link(<core-Ref>)[`Ref`] → `core.stoppingPlaces`], [die Halteplätze an ihm],
)

=== Bridge <civil-Bridge>

Eine Eisenbahnüberführung, über die das Gleis führt: von ihrem Anfang bis zu ihrem Ende entlang des Gleises.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt des Gleises],
  [`kind`], [], [eins der Werte unter Bedeutung], [Tragwerk #linebreak() `beam` Balkenbrücke #linebreak() `arch` Bogenbrücke #linebreak() `frame` Rahmenbrücke #linebreak() `truss` Fachwerkbrücke #linebreak() `other` sonstige],
  [`spans`], [], [ganze Zahl; ≥ 1], [Anzahl der Felder],
  [`crosses`], [], [#link(<core-Text>)[`Text`]], [was sie überführt (Gewässer, Straße, …)],
  [`clearance`], [], [Zahl; ≥ 0], [lichte Höhe darunter \[m\]],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== RetainingWall <civil-RetainingWall>

Eine Stützwand neben dem Gleis.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt und Seite des Gleises],
  [`kind`], [], [eins der Werte unter Bedeutung], [Bauart #linebreak() `cantilever` Winkelstützmauer #linebreak() `gravity` Schwergewichtsmauer #linebreak() `sheetPile` Spundwand #linebreak() `gabion` Gabionen #linebreak() `other` sonstige],
  [`offset`], [], [Zahl; ≥ 0], [Abstand von der Gleisachse \[m\]],
  [`height`], [], [Zahl; ≥ 0], [Höhe \[m\]],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== NoiseBarrier <civil-NoiseBarrier>

Eine Lärmschutzwand neben dem Gleis.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt und Seite des Gleises],
  [`kind`], [], [eins der Werte unter Bedeutung], [Bauart #linebreak() `absorbing` hochabsorbierend #linebreak() `reflecting` reflektierend #linebreak() `transparent` transparent],
  [`offset`], [], [Zahl; ≥ 0], [Abstand von der Gleisachse \[m\]],
  [`height`], [], [Zahl; ≥ 0], [Höhe über SO \[m\]],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

== Layer signalling 3.1

=== Layerdokument <signalling-root>

Der Layer signalling: die Leit- und Sicherungstechnik.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`fields`], [ja], [#link(<signalling-Fields>)[`Fields`]], [Die Felder des Layers],
  [`collections`], [ja], [Objekt (Properties unten)], [Die Sammlungen des Layers],
  [`collections.signals`], [ja], [Liste von #link(<signalling-Signal>)[`Signal`]], [Signale],
  [`collections.detectionPoints`], [ja], [Liste von #link(<signalling-DetectionPoint>)[`DetectionPoint`]], [Freimeldepunkte],
  [`collections.trackSections`], [ja], [Liste von #link(<signalling-TrackSection>)[`TrackSection`]], [Freimeldeabschnitte],
  [`collections.switchElements`], [ja], [Liste von #link(<signalling-SwitchElement>)[`SwitchElement`]], [Weichen- und Gleissperrenelemente],
  [`collections.routes`], [ja], [Liste von #link(<signalling-Route>)[`Route`]], [Fahrstraßen],
  [`collections.overlaps`], [ja], [Liste von #link(<signalling-Overlap>)[`Overlap`]], [Durchrutschwege],
  [`collections.flankProtection`], [ja], [Liste von #link(<signalling-FlankProtection>)[`FlankProtection`]], [Flankenschutz],
  [`collections.balises`], [ja], [Liste von #link(<signalling-Balise>)[`Balise`]], [Magnete und Balisen],
  [`collections.stationBoundaries`], [ja], [Liste von #link(<signalling-StationBoundary>)[`StationBoundary`]], [Bahnhofsgrenzen],
  [`collections.itineraries`], [ja], [Liste von #link(<signalling-Itinerary>)[`Itinerary`]], [Fahrwege],
  [`collections.trainProtection`], [], [Liste von #link(<signalling-TrainProtection>)[`TrainProtection`]], [linienförmige Zugbeeinflussung über Gleisabschnitte],
)

=== Fields <signalling-Fields>

Die Felder des Layers: der Signalbegriffs-Katalog, gegen den geplant wurde, und die Zeiten der Fahrstraßen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`catalogue`], [], [Objekt (Properties unten)], [der Katalog der Signalbegriffe],
  [`catalogue.id`], [ja], [Text; mindestens 1 Zeichen], [Kennung des Katalogs],
  [`catalogue.version`], [ja], [Text; mindestens 1 Zeichen], [Version des Katalogs],
  [`routeTimes`], [], [Objekt (Properties unten)], [Zeiten, die für jede Fahrstraße gelten, die keine eigenen nennt],
  [`routeTimes.setting`], [], [Zahl; ≥ 0], [Fahrstraßenbildezeit \[s\]],
  [`routeTimes.release`], [], [Zahl; ≥ 0], [Auflösezeit \[s\]],
)

=== Directed <signalling-Directed>

Ein Ort, der seine Wirkrichtung angibt: `direction` auf einem Gleis, `bearing` in der Ebene.

Wert: ein #link(<core-TrackLocation>)[`TrackLocation`] mit mindestens einem von `direction`, `bearing`

=== SwitchPosition <signalling-SwitchPosition>

Lage einer Weiche, von der Spitze aus gesehen (PlanPro: L, R).

Wert: eins von `left`, `right`: #linebreak() `left` links (L) #linebreak() `right` rechts (R)

=== Signal <signalling-Signal>

Ein Signalstandort, das physische Signal (PlanPro Signal), real oder fiktiv. Ein reales trägt je Signalsystem ein logisches Signal und eigene Tafeln; ein fiktives (PlanPro Signal\_Fiktiv) markiert, wo eine Fahrstraße ohne Signal beginnt oder endet.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Label>)[`Label`]], [Name (ohne Kennzahl)],
  [`areaNumber`], [], [#link(<core-Label>)[`Label`]], [Kennzahl des Stellbereichs, die der vollen Bezeichnung vorangeht (PlanPro Kennzahl: 60 in 60A)],
  [`operationalPointId`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [Bahnhof, zu dem es gehört; ohne: Signal der freien Strecke],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [real oder fiktiv #linebreak() `real` reales Signal #linebreak() `virtual` fiktives Signal],
  [`at`], [ja], [#link(<signalling-Directed>)[`Directed`]], [Standort und Wirkrichtung],
  [`alsoAt`], [], [Liste von #link(<core-TrackLocation>)[`TrackLocation`]], [weitere Gleise am selben Ort (Grenzzeichen zwischen zwei Gleisen)],
  [`bothDirections`], [], [Wahrheitswert], [wirkt in beide Richtungen],
  [`function`], [], [eins der Werte unter Bedeutung], [Funktion eines Signals ohne System (PlanPro Signal\_Funktion) #linebreak() `entry` Einfahrsignal #linebreak() `exit` Ausfahrsignal #linebreak() `entryExit` Ein- und Ausfahrsignal #linebreak() `intermediate` Zwischensignal #linebreak() `exitIntermediate` Ausfahr- und Zwischensignal #linebreak() `entryBlock` Einfahr- und Blocksignal #linebreak() `block` Blocksignal #linebreak() `groupExit` Gruppenausfahrsignal #linebreak() `groupIntermediate` Gruppenzwischensignal #linebreak() `groupExitIntermediate` Gruppenausfahr- und Zwischensignal #linebreak() `followUp` Folgesignal #linebreak() `cover` Deckungssignal #linebreak() `trainCover` Zugdeckungssignal #linebreak() `trainTarget` Zugzielsignal #linebreak() `levelCrossing` Überwachungssignal Bahnübergang #linebreak() `distant` Vorsignal #linebreak() `standaloneIndicator` alleinstehender Anzeiger (Tafel) #linebreak() `other` sonstige],
  [`virtualFunction`], [], [eins der Werte unter Bedeutung], [Funktion eines fiktiven Signals (PlanPro Fiktives\_Signal\_Funktion) #linebreak() `trainStartWithoutSignal` Zugstart ohne Signal #linebreak() `trainTargetWithoutSignal` Zugziel ohne Signal #linebreak() `trainTargetLine` Zugziel Strecke #linebreak() `trainStartSiding` Zugstart Awanst #linebreak() `trainTargetSiding` Zugziel Awanst #linebreak() `trainStartMiddleSwitch` Zugstart Mittelweiche #linebreak() `trainStartTargetBlock` Zugstart/-ziel Bk #linebreak() `trainStartTargetBlockZs1` Zugstart/-ziel Bk mit Zs 1 #linebreak() `trainStartTargetBlockZs7` Zugstart/-ziel Bk mit Zs 7 #linebreak() `shuntingStartTargetWithoutSignal` Rangierstart/-ziel ohne Signal #linebreak() `virtualDistant` Vorsignal fiktiv #linebreak() `other` sonstige],
  [`logicalSignals`], [], [Liste von #link(<signalling-LogicalSignal>)[`LogicalSignal`]], [die logischen Signale, eines je Signalsystem],
  [`mounting`], [], [eins der Werte unter Bedeutung], [Befestigungsart (PlanPro Signal\_Befestigungsart) #linebreak() `mast` Mast #linebreak() `bracket` Signalausleger #linebreak() `gantry` Signalbrücke #linebreak() `foundation` Fundament #linebreak() `special` Sonderkonstruktion #linebreak() `other` andere],
  [`foundation`], [], [Objekt (Properties unten)], [Fundament eines Signals auf Signalbrücke oder -ausleger],
  [`foundation.side`], [], [#link(<core-Side>)[`Side`]], [Seite, gesehen in Wirkrichtung],
  [`foundation.offset`], [ja], [Zahl; ≥ 0], [Abstand von der Gleisachse \[m\]],
  [`sight`], [], [Objekt (Properties unten)], [Signalsicht \[m\] (PlanPro Signalsicht\_Soll, \_Mindest, \_Erreichbar)],
  [`sight.required`], [], [Zahl; ≥ 0], [erforderlich],
  [`sight.minimum`], [], [Zahl; ≥ 0], [mindestens],
  [`sight.achievable`], [], [Zahl; ≥ 0], [erreichbar],
  [`frames`], [], [Liste von #link(<signalling-Frame>)[`Frame`]], [was es ohne System zeigt: Tafeln, Grenzzeichen-Körper, Namensschild, Schirm eines Signals ohne System],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== LogicalSignal <signalling-LogicalSignal>

Ein Signal in einem Signalsystem an einem Standort: System, Signalart, Funktion, Rahmen, die signalisierte Geschwindigkeit je Fahrstraße, Einstellungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`system`], [ja], [Text; mindestens 1 Zeichen], [Signalsystem, ein Schlüssel des Katalogs (z. B. `ks`, `sh`)],
  [`type`], [], [Text; mindestens 1 Zeichen], [Signalart im System, ein Schlüssel des Katalogs (z. B. `main`) oder `other`],
  [`function`], [], [eins der Werte unter Bedeutung], [Funktion (PlanPro Signal\_Funktion) #linebreak() `entry` Einfahrsignal #linebreak() `exit` Ausfahrsignal #linebreak() `entryExit` Ein- und Ausfahrsignal #linebreak() `intermediate` Zwischensignal #linebreak() `exitIntermediate` Ausfahr- und Zwischensignal #linebreak() `entryBlock` Einfahr- und Blocksignal #linebreak() `block` Blocksignal #linebreak() `groupExit` Gruppenausfahrsignal #linebreak() `groupIntermediate` Gruppenzwischensignal #linebreak() `groupExitIntermediate` Gruppenausfahr- und Zwischensignal #linebreak() `followUp` Folgesignal #linebreak() `cover` Deckungssignal #linebreak() `trainCover` Zugdeckungssignal #linebreak() `trainTarget` Zugzielsignal #linebreak() `levelCrossing` Überwachungssignal Bahnübergang #linebreak() `distant` Vorsignal #linebreak() `standaloneIndicator` alleinstehender Anzeiger (Tafel) #linebreak() `other` sonstige],
  [`frames`], [], [Liste von #link(<signalling-Frame>)[`Frame`]], [Schirm und Zusatzanzeiger],
  [`signallingSpeeds`], [], [Liste von Objekten (Properties unten)], [signalisierte Geschwindigkeit je Fahrstraße, die an diesem Standort beginnt (PlanPro Fstr\_V\_Hg)],
  [`signallingSpeeds[].routeId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.routes`], [die Fahrstraße],
  [`signallingSpeeds[].speed`], [ja], [Zahl; ≥ 0], [Geschwindigkeit \[km/h\]],
  [`settings`], [], [#link(<signalling-Settings>)[`Settings`]], [Einstellungen nach dem Katalog],
  [`nextSystems`], [], [Liste von Text; mindestens 1 Zeichen], [Systeme, die die angekündigten Signale haben dürfen],
  [`defaultParameters`], [], [#link(<signalling-Settings>)[`Settings`]], [Parameter, mit denen es standardmäßig zeigt],
  [`conditionalParameters`], [], [Liste von Objekten (Properties unten)], [Parameter je Fahrstraße],
  [`conditionalParameters[].routeId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.routes`], [die Fahrstraße],
  [`conditionalParameters[].parameters`], [ja], [#link(<signalling-Settings>)[`Settings`]], [ihre Parameter],
)

=== Settings <signalling-Settings>

Werte, die ein System liest, unter den Namen, die der Katalog für System und Art festlegt: Auswahl, Zahl, Wahrheitswert oder Text.

Wert: Objekt; Schlüssel frei, Werte: Text oder Zahl oder Wahrheitswert

=== Frame <signalling-Frame>

Ein Teil eines Signals, der Begriffe zeigt (PlanPro Signal\_Rahmen).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art des Rahmens (PlanPro Rahmen\_Art) #linebreak() `screen` Schirm #linebreak() `indicator` Zusatzanzeiger #linebreak() `plate` Blechtafel #linebreak() `body` Keramikkörper #linebreak() `nameplate` Bezeichnungsschild #linebreak() `other` sonstige],
  [`aspects`], [ja], [Liste von #link(<signalling-Aspect>)[`Aspect`]], [die Begriffe, die er zeigen kann],
)

=== Aspect <signalling-Aspect>

Ein Begriff, den ein Rahmen zeigen kann (PlanPro Signal\_Signalbegriff), mit Symbol, wo der Katalog eines verlangt.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`aspect`], [ja], [Text; mindestens 1 Zeichen], [Schlüssel des Begriffs im Katalog, der PlanPro-Name (`Hp_0`, `Ks_1`, `Zs_3`, …)],
  [`symbol`], [], [Text; mindestens 1 Zeichen], [Symbol, wo der Katalog eines verlangt: bei Zs 3 und Lf 7 die Geschwindigkeit in Zehnern km/h (`7` = 70 km/h), bei Zs 2 der Buchstabe],
  [`lit`], [], [Wahrheitswert], [beleuchtet],
  [`switched`], [], [Wahrheitswert], [schaltbar],
)

=== DetectionPoint <signalling-DetectionPoint>

Ein Punkt, der Gleisfreimeldeabschnitte begrenzt (PlanPro FMA\_Komponente).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Label>)[`Label`]], [Name],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art #linebreak() `axleCounter` Achszählpunkt #linebreak() `insulatedJointOneRail` Isolierstoß einschienig #linebreak() `insulatedJointTwoRail` Isolierstoß zweischienig #linebreak() `electricalJoint` elektrischer Stoß],
  [`at`], [ja], [#link(<core-TrackLocation>)[`TrackLocation`]], [Standort auf dem Gleis],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== TrackSection <signalling-TrackSection>

Ein Gleisfreimeldeabschnitt (PlanPro FMA\_Anlage). Seine Ausdehnung wird nicht gespeichert, sondern aus den begrenzenden Punkten abgeleitet.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Label>)[`Label`]], [Name],
  [`areaNumber`], [], [#link(<core-Label>)[`Label`]], [Kennzahl des Stellbereichs, die der vollen Bezeichnung vorangeht (PlanPro Kennzahl: 60 in 60A)],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art der Freimeldung #linebreak() `axleCounting` Achszählung #linebreak() `audioFrequencyCircuit` Tonfrequenz-Gleisstromkreis #linebreak() `lowFrequencyCircuit` Niederfrequenz-Gleisstromkreis #linebreak() `ftgs` FTGS #linebreak() `other` sonstige],
  [`boundedBy`], [ja], [Liste von #link(<core-Ref>)[`Ref`] → `signalling.detectionPoints`; mindestens 1 Eintrag], [die begrenzenden Freimeldepunkte],
  [`auxiliaryClearing`], [], [Wahrheitswert], [Hilfsfreimeldung (PlanPro FMA\_Hilffreimeldung)],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== SwitchElement <signalling-SwitchElement>

Eine Weiche des Kerns oder eine Gleissperre, wie das Stellwerk sie kennt (PlanPro W\_Kr\_Gsp\_Element). Eine Kreuzungsweiche hat zwei.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Label>)[`Label`]], [Name],
  [`areaNumber`], [], [#link(<core-Label>)[`Label`]], [Kennzahl des Stellbereichs, die der vollen Bezeichnung vorangeht (PlanPro Kennzahl: 60 in 60A)],
  [`switchId`], [], [#link(<core-Ref>)[`Ref`] → `core.switches`], [die Weiche des Kerns],
  [`derailer`], [], [Objekt (Properties unten)], [eine Gleissperre (PlanPro Gleissperre\_Element)],
  [`derailer.at`], [ja], [#link(<signalling-Directed>)[`Directed`]], [Standort],
  [`derailer.preferredPosition`], [], [eins der Werte unter Bedeutung], [Grundstellung #linebreak() `applied` aufgelegt #linebreak() `removed` abgelegt],
  [`part`], [], [eins der Werte unter Bedeutung], [welches Zungenpaar einer Kreuzungsweiche #linebreak() `a` Zungenpaar a #linebreak() `b` Zungenpaar b],
  [`operation`], [ja], [eins der Werte unter Bedeutung], [Stellart (PlanPro W\_Kr\_Gsp\_Stellart) #linebreak() `electricRemote` elektrisch ferngestellt #linebreak() `electricLocal` elektrisch ortsgestellt #linebreak() `mechanicalRemote` mechanisch ferngestellt #linebreak() `mechanicalLocal` mechanisch ortsgestellt #linebreak() `notWorked` nicht gestellt #linebreak() `springBack` Rückfallweiche #linebreak() `disabledLeft` außer Betrieb links #linebreak() `disabledRight` außer Betrieb rechts #linebreak() `other` sonstige],
  [`preferredPosition`], [], [#link(<signalling-SwitchPosition>)[`SwitchPosition`]], [Vorzugslage],
  [`preferredAutomatic`], [], [Wahrheitswert], [kehrt selbst in die Vorzugslage zurück],
  [`trailable`], [], [Wahrheitswert], [auffahrbar (PlanPro Auffahrortung)],
  [`clearanceMarkerId`], [], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [Grenzzeichen (Ra 12) der Weiche, ein Signal des Layers],
  [`twofoldProtection`], [], [Objekt (Properties unten)], [Zwieschutz (PlanPro Fla\_Zwieschutz)],
  [`twofoldProtection.kind`], [], [eins der Werte unter Bedeutung], [Art des Zwieschutzes #linebreak() `genuine` echter Zwieschutz #linebreak() `genuineOwn` echter und Eigenzwieschutz #linebreak() `own` Eigenzwieschutz],
  [`twofoldProtection.left`], [], [eins der Werte unter Bedeutung], [Maßnahme für die Lage links #linebreak() `locking` Verschluss #linebreak() `substitute` Ersatzmaßnahme],
  [`twofoldProtection.right`], [], [eins der Werte unter Bedeutung], [Maßnahme für die Lage rechts #linebreak() `locking` Verschluss #linebreak() `substitute` Ersatzmaßnahme],
  [`twofoldProtection.preventsRunOn`], [], [Wahrheitswert], [verhindert das Weiterfahren],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== Route <signalling-Route>

Eine Zug-, Rangier- oder Mittelweichenstraße (PlanPro Fstr\_Zug\_Rangier mit Fstr\_Fahrweg) von einem Start- zu einem Zielsignal über eine Gleisfolge. Die Weichenlagen folgen aus der Gleisfolge.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art der Fahrstraße #linebreak() `train` Zugstraße #linebreak() `shunting` Rangierstraße #linebreak() `middle` Mittelweichenstraße],
  [`trainType`], [], [eins von `B`, `ZH`, `ZR`, `ZT`, `ZTU`, `ZU`, `ZUH`, `ZZ`], [Art der Zugstraße (PlanPro Fstr\_Zug\_Art)],
  [`shuntingType`], [], [eins von `RR`, `RT`, `RTU`, `RU`], [Art der Rangierstraße (PlanPro Fstr\_Rangier\_Art)],
  [`middleType`], [], [eins von `ZM`, `ZUM`], [Art der Mittelweichenstraße (PlanPro Fstr\_Mittel\_Art)],
  [`rank`], [], [ganze Zahl; ≥ 0], [Rang unter den Fahrstraßen zwischen denselben Signalen, 0 die Regelfahrstraße],
  [`startSignalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [Startsignal],
  [`targetSignalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [Zielsignal],
  [`path`], [ja], [#link(<core-TrackPath>)[`TrackPath`]], [die Gleisfolge],
  [`autoSetting`], [], [Wahrheitswert], [durch Selbststellbetrieb eingestellt],
  [`shortDistantSpacing`], [], [Wahrheitswert], [Vorsignal im verkürzten Bremswegabstand],
  [`clearSections`], [], [Liste von #link(<core-Ref>)[`Ref`] → `signalling.trackSections`], [Gleisfreimeldeabschnitte, die frei sein müssen],
  [`overlaps`], [], [Liste von Objekten (Properties unten)], [Durchrutschwege, in denen sie enden darf],
  [`overlaps[].overlapId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.overlaps`], [der Durchrutschweg],
  [`overlaps[].preferred`], [], [Wahrheitswert], [der bevorzugte],
  [`aspects`], [], [Liste von Objekten (Properties unten)], [was die Fahrstraße zeigt: an welchem Signal welcher Begriff (PlanPro Fstr\_Signalisierung)],
  [`aspects[].signalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [das Signal],
  [`aspects[].aspect`], [ja], [Text; mindestens 1 Zeichen], [der Begriff],
  [`aspects[].symbol`], [], [Text; mindestens 1 Zeichen], [sein Symbol],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
  [`settingTime`], [], [Zahl; ≥ 0], [Fahrstraßenbildezeit \[s\]; ohne sie die des Layers],
  [`releaseTime`], [], [Zahl; ≥ 0], [Auflösezeit, nachdem der Zug sie verlassen hat \[s\]; ohne sie die des Layers],
)

=== Overlap <signalling-Overlap>

Ein Durchrutschweg hinter dem Zielsignal (PlanPro Fstr\_DWeg) bis zum Gefahrpunkt.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`signalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [das Signal, hinter dem er liegt],
  [`path`], [ja], [#link(<core-TrackPath>)[`TrackPath`]], [die Gleisfolge],
  [`end`], [ja], [#link(<core-TrackLocation>)[`TrackLocation`]], [Gefahrpunkt (PlanPro ID\_PZB\_Gefahrpunkt)],
  [`length`], [ja], [Zahl; ≥ 0], [Soll-Länge \[m\]],
  [`gradient`], [], [Zahl], [maßgebende Neigung \[‰\], negativ im Gefälle],
  [`speed`], [], [Zahl; ≥ 0], [zulässige Geschwindigkeit \[km/h\]],
  [`releaseDelay`], [], [Zahl; ≥ 0], [Auflösungsverzögerung \[s\]],
  [`clearSections`], [], [Liste von #link(<core-Ref>)[`Ref`] → `signalling.trackSections`], [Gleisfreimeldeabschnitte, die frei sein müssen],
  [`switches`], [], [Liste von Objekten (Properties unten)], [Weichen im Durchrutschweg],
  [`switches[].switchElementId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.switchElements`], [das Weichenelement],
  [`switches[].locked`], [ja], [Wahrheitswert], [verschlossen],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== FlankProtection <signalling-FlankProtection>

Der Flankenschutz, den ein Weichenelement verlangt, wenn es in einer Lage befahren wird (PlanPro Fla\_Schutz).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`requester`], [ja], [Objekt (Properties unten)], [wer ihn verlangt],
  [`requester.switchElementId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.switchElements`], [das Weichenelement],
  [`requester.via`], [ja], [eins der Werte unter Bedeutung], [befahren in welcher Lage, oder von der Spitze #linebreak() `left` in Lage links #linebreak() `right` in Lage rechts #linebreak() `tip` von der Spitze],
  [`protection`], [ja], [eins von: Objekt (Properties unten); Objekt (Properties unten); Objekt (Properties unten)], [wer ihn gibt: ein Signal in Haltstellung, ein Weichenelement in einer Lage, oder verzichtet],
  [`protection.signalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [das schützende Signal],
  [`protection.targetLock`], [], [Wahrheitswert], [Zielsperrung (PlanPro Fla\_Signal\_Zielsperrung)],
  [`protection.switchElementId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.switchElements`], [das schützende Weichenelement],
  [`protection.position`], [], [#link(<signalling-SwitchPosition>)[`SwitchPosition`]], [seine Lage; eine Gleissperre gibt ihn aufgelegt und nennt keine],
  [`protection.waived`], [ja], [genau `true`], [auf Flankenschutz verzichtet],
  [`passOn`], [], [Objekt (Properties unten)], [Flankenschutz, den das schützende Element seinerseits verlangt (PlanPro Fla\_Schutz\_Weitergabe)],
  [`passOn.left`], [], [#link(<core-Ref>)[`Ref`] → `signalling.flankProtection`], [für seine Lage links],
  [`passOn.right`], [], [#link(<core-Ref>)[`Ref`] → `signalling.flankProtection`], [für seine Lage rechts],
  [`clearSections`], [], [Liste von #link(<core-Ref>)[`Ref`] → `signalling.trackSections`], [Gleisfreimeldeabschnitte, die frei sein müssen],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== Balise <signalling-Balise>

Ein punktförmiges Element der Zugbeeinflussung: PZB-Magnet, Geschwindigkeitsprüfung, ETCS-Balise (PlanPro PZB\_Element, Balise).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Label>)[`Label`]], [Name],
  [`at`], [ja], [#link(<signalling-Directed>)[`Directed`]], [Standort und Wirkrichtung],
  [`system`], [ja], [Text; mindestens 1 Zeichen], [System der Zugbeeinflussung, ein Schlüssel des Katalogs (`pzb`, `etcs`)],
  [`kind`], [ja], [Text; mindestens 1 Zeichen], [Art im System, ein Schlüssel des Katalogs],
  [`settings`], [], [#link(<signalling-Settings>)[`Settings`]], [Einstellungen nach dem Katalog],
  [`signalId`], [], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [zugehöriges Signal (PlanPro PZB\_Element\_Bezugspunkt)],
  [`effect`], [ja], [eins der Werte unter Bedeutung], [Wirksamkeit (PlanPro Wirksamkeit) #linebreak() `permanent` dauernd wirksam #linebreak() `switchedBySignal` vom Signal geschaltet #linebreak() `other` sonstige],
  [`routes`], [], [Liste von Objekten (Properties unten)], [Fahrstraßen, die es schalten (PlanPro PZB\_Element\_Zuordnung\_Fstr)],
  [`routes[].routeId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.routes`], [die Fahrstraße],
  [`routes[].effect`], [ja], [eins der Werte unter Bedeutung], [wie sie es schaltet #linebreak() `activeWhenSet` wirksam bei eingestellter Fahrstraße #linebreak() `inactiveWhenSet` unwirksam bei eingestellter Fahrstraße #linebreak() `other` sonstige],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== StationBoundary <signalling-StationBoundary>

Die Grenze eines Bahnhofs, wo Züge ihn betreten: Einfahrsignal oder Tafel (Ne 1), sonst die Einfahrweiche, bei einem Haltepunkt sein Halteplatz.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`operationalPointId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [der Bahnhof],
  [`marks`], [ja], [Liste von eins der Objekte unten; mindestens 1 Eintrag], [was die Grenze markiert: ein Signal, eine Weiche oder ein Halteplatz],
  [`marks[].signalId`], [ja], [#link(<core-Ref>)[`Ref`] → `signalling.signals`], [Einfahrsignal oder Tafel],
  [`marks[].switchId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.switches`], [Einfahrweiche],
  [`marks[].stoppingPlaceId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.stoppingPlaces`], [Halteplatz eines Haltepunkts],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== Itinerary <signalling-Itinerary>

Ein Fahrweg von einem Halteplatz zu einem anderen: seine Gleisfolge und die Zugstraßen, die er nimmt.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`startStoppingPlaceId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.stoppingPlaces`], [Start-Halteplatz],
  [`targetStoppingPlaceId`], [ja], [#link(<core-Ref>)[`Ref`] → `core.stoppingPlaces`], [Ziel-Halteplatz],
  [`routeIds`], [ja], [Liste von #link(<core-Ref>)[`Ref`] → `signalling.routes`], [die Zugstraßen in Fahrtrichtung],
  [`path`], [ja], [#link(<core-TrackPath>)[`TrackPath`]], [die Gleisfolge],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

=== TrainProtection <signalling-TrainProtection>

Die Zugbeeinflussung über einen Abschnitt eines Gleises: ihr System und, bei ETCS, ihr Level.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt des Gleises],
  [`system`], [ja], [eins der Werte unter Bedeutung], [System #linebreak() `pzb` PZB #linebreak() `lzb` LZB #linebreak() `etcs` ETCS],
  [`level`], [], [eins von `1`, `2`, `3`], [ETCS-Level],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

== Layer electrification 2.1

=== Layerdokument <electrification-root>

Der Layer electrification: die Bereiche unter Fahrleitung und ihr System.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`fields`], [ja], [#link(<electrification-Fields>)[`Fields`]], [Die Felder des Layers],
  [`collections`], [ja], [Objekt (Properties unten)], [Die Sammlungen des Layers],
  [`collections.electrifications`], [ja], [Liste von #link(<electrification-Electrification>)[`Electrification`]], [elektrifizierte Abschnitte],
)

=== Fields <electrification-Fields>

Die Felder des Layers (noch keine).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
)

=== Electrification <electrification-Electrification>

Ein Abschnitt eines Gleises unter Fahrleitung eines Systems; ein Zug fährt dort elektrisch, wo sein Fahrzeug dieses System hat.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [#link(<core-Text>)[`Text`]], [Name],
  [`at`], [ja], [#link(<core-TrackRange>)[`TrackRange`]], [Abschnitt des Gleises],
  [`system`], [ja], [eins der Werte unter Bedeutung], [Spannung, wie OSRD die Traktionsart nennt #linebreak() `15000V` 15 kV 16,7 Hz #linebreak() `25000V` 25 kV 50 Hz #linebreak() `3000V` 3 kV Gleichstrom #linebreak() `1500V` 1,5 kV Gleichstrom #linebreak() `750V` 750 V Gleichstrom #linebreak() `600V` 600 V Gleichstrom],
  [`lifespan`], [], [#link(<core-Lifespan>)[`Lifespan`]], [Lebensdauer: die Tage im Betrieb; fehlt sie, besteht das Objekt und bleibt],
)

== Hülle von operations 1.0

=== Datei <operations-file-root>

Die Fahrplandatei: Format, Version, die Domäne operations, mitgeführte Schemas und Blobs — dieselbe Hülle wie infra-data.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`format`], [ja], [genau `operations`], [Kennung des Formats],
  [`version`], [ja], [genau `1.0`], [Version der Hülle],
  [`domains`], [ja], [Objekt (Properties unten)], [Die Domänen; genau die eine Domäne `operations`],
  [`domains.operations`], [ja], [#link(<operations-file-Domain>)[`Domain`]], [der Betrieb des Projekts],
  [`schemas`], [], [Objekt; Schlüssel nach `^sha256:[0-9a-f]{64}$`: Objekt], [Das Schema der Domäne, unter der Prüfsumme, mit der sie es nennt],
  [`blobs`], [ja], [Objekt; Schlüssel nach `^sha256:[0-9a-f]{64}$`: beliebiger Wert], [Große Daten (die Fahrzeuge als RailJSON-Rollmaterial), je unter der Prüfsumme ihres kanonischen JSON],
)

=== Domain <operations-file-Domain>

Die Domäne in der Datei: ihre Version, ihr Schema, ihre Felder und Sammlungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`version`], [ja], [Text; Muster `^[0-9]+\.[0-9]+$`], [Version der Domäne],
  [`schema`], [ja], [Text; Muster `^sha256:[0-9a-f]{64}$`], [Prüfsumme des Schemas, gegen das die Domäne geschrieben ist],
  [`dependsOn`], [], [Liste von Text], [Domänen, auf die sie verweist],
  [`fields`], [ja], [Objekt], [Die eigenen Felder der Domäne],
  [`collections`], [ja], [Objekt; Schlüssel frei, Werte: Liste von Objekt], [Die Sammlungen, je eine Liste von Objekten mit `id`],
)

== Domäne operations 1.0

=== Domänendokument <operations-root>

Die Domäne operations: der ganze Betrieb eines Projekts. Sie gehört zu keiner Variante und nennt die Infrastruktur über Kennungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`fields`], [ja], [#link(<operations-Fields>)[`Fields`]], [Die Felder der Domäne],
  [`collections`], [ja], [Objekt (Properties unten)], [Die Sammlungen der Domäne],
  [`collections.timetables`], [ja], [Liste von #link(<operations-Timetable>)[`Timetable`]], [Fahrpläne],
  [`collections.trains`], [ja], [Liste von #link(<operations-Train>)[`Train`]], [Züge],
  [`collections.trainKinds`], [ja], [Liste von #link(<operations-TrainKind>)[`TrainKind`]], [Zuggattungen],
  [`collections.formations`], [ja], [Liste von #link(<operations-Formation>)[`Formation`]], [Zugbildungen],
  [`collections.vehicles`], [ja], [Liste von #link(<operations-Vehicle>)[`Vehicle`]], [mitgeführte Fahrzeuge],
  [`collections.connections`], [ja], [Liste von #link(<operations-Connection>)[`Connection`]], [Anschlüsse],
  [`collections.turnarounds`], [ja], [Liste von #link(<operations-Turnaround>)[`Turnaround`]], [Wenden],
  [`collections.delayScenarios`], [ja], [Liste von #link(<operations-DelayScenario>)[`DelayScenario`]], [Verspätungsszenarien],
  [`collections.studies`], [ja], [Liste von #link(<operations-Study>)[`Study`]], [Kapazitätsuntersuchungen],
)

=== Fields <operations-Fields>

Die Felder der Domäne (noch keine).

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
)

=== Timetable <operations-Timetable>

Ein Fahrplan; seine Züge nennen ihn.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [ja], [Text; mindestens 1 Zeichen], [Name],
  [`dispatching`], [], [#link(<operations-Dispatching>)[`Dispatching`]], [was geschieht, wo Züge sich begegnen],
  [`blockingTimes`], [], [#link(<operations-BlockingTimes>)[`BlockingTimes`]], [die Teile der Sperrzeit, die der Betrieb vorgibt],
)

=== Dispatching <operations-Dispatching>

Was geschieht, wo Züge sich begegnen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`mode`], [], [eins der Werte unter Bedeutung], [Art der Disposition #linebreak() `show` nur die Konflikte zeigen, jeder Zug fährt für sich #linebreak() `wait` die Züge nacheinander ins Netz geben, jeder wartet auf einen freien Abschnitt],
  [`priority`], [], [eins der Werte unter Bedeutung], [in welcher Reihenfolge die Züge ins Netz gegeben werden #linebreak() `departure` nach Abfahrt #linebreak() `kind` nach Zuggattung (`kindOrder`)],
  [`kindOrder`], [], [Liste von #link(<core-Ref>)[`Ref`] → `operations.trainKinds`; höchstens 100 Einträge], [die Zuggattungen, die zuerst ins Netz gegeben werden, in ihrer Reihenfolge],
)

=== BlockingTimes <operations-BlockingTimes>

Die Teile der Sperrzeit, die der Betrieb vorgibt: was gilt, wo die Infrastruktur nichts sagt, und die Sichtzeit.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`setup`], [], [Zahl; ≥ 0; ≤ 10000], [Fahrstraßenbildezeit \[s\], wo die Infrastruktur keine nennt],
  [`approach`], [], [Zahl; ≥ 0; ≤ 10000], [wie weit vor seinem Signal ein Abschnitt angefordert wird \[m\], wo kein Signal ihn ankündigt],
  [`release`], [], [Zahl; ≥ 0; ≤ 10000], [Auflösezeit \[s\], wo die Infrastruktur keine nennt],
  [`sight`], [], [Zahl; ≥ 0; ≤ 10000], [Sichtzeit des Triebfahrzeugführers vor dem ankündigenden Signal \[s\]],
)

=== Train <operations-Train>

Ein Zug eines Fahrplans.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`timetableId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.timetables`], [der Fahrplan],
  [`number`], [], [Text], [Zugnummer],
  [`name`], [], [Text], [Name],
  [`trainKindId`], [], [#link(<core-Ref>)[`Ref`] → `operations.trainKinds`], [die Zuggattung],
  [`formationId`], [], [#link(<core-Ref>)[`Ref`] → `operations.formations`], [die Zugbildung],
  [`mode`], [], [Text], [Traktionsart; ohne sie die, die die Fahrleitung auf seinem Weg zulässt],
  [`comfort`], [], [Text], [Komfortstufe nach OSRD: STANDARD, AIR\_CONDITIONING, HEATING],
  [`departure`], [ja], [Zahl; ≥ 0; ≤ 172799.999], [Abfahrt in Sekunden ab Mitternacht des Fahrplantags; über 24:00 der nächste Tag],
  [`margin`], [], [Text], [Fahrzeitzuschlag, wie OSRD ihn schreibt: `5 %`, `3 min/100km`],
  [`distribution`], [], [eins der Werte unter Bedeutung], [wie der Zuschlag verteilt wird #linebreak() `mareco` energiesparend (MARECO) #linebreak() `linear` gleichmäßig],
  [`points`], [ja], [Liste von #link(<operations-TrainPoint>)[`TrainPoint`]\; höchstens 300 Einträge], [wo er fährt: vom ersten zum letzten Punkt, über die anderen der Reihe nach],
)

=== TrainPoint <operations-TrainPoint>

Ein Punkt eines Zugs: ein Ort, ob er dort hält, eine Abfahrt, auf die er wartet.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`place`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [der Ort über seine Kennung: eine Betriebsstelle des Verzeichnisses, oder ein Prellbock oder eine Betrachtungsgrenze (core.endMarks)],
  [`station`], [ja], [Text; mindestens 1 Zeichen], [der Name, unter dem der Ort gewählt wurde — was einer Variante gesagt wird, der er fehlt],
  [`track`], [], [Text], [das Gleis, auf dem er dort fährt, über seine Kennung (core.tracks); ohne: beliebig],
  [`stop`], [ja], [Wahrheitswert], [hält dort],
  [`dwell`], [], [Zahl; ≥ 0; ≤ 86400], [Haltezeit \[s\]],
  [`leastDwell`], [], [Zahl; ≥ 0; ≤ 86400], [Mindesthaltezeit \[s\], die die Disposition einhält],
  [`departure`], [], [Zahl; ≥ 0; ≤ 172799.999], [Abfahrt in Sekunden ab Mitternacht des Fahrplantags; über 24:00 der nächste Tag],
  [`itinerary`], [], [#link(<core-Ref>)[`Ref`] → `signalling.itineraries`], [der Fahrweg, den er zum nächsten Punkt bevorzugt],
)

=== TrainKind <operations-TrainKind>

Eine Zuggattung.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [ja], [Text], [Name],
  [`rank`], [], [ganze Zahl; ≥ 0], [Rang in der Disposition: je kleiner, desto früher ins Netz gegeben],
  [`color`], [], [Text; Muster `^#[0-9a-fA-F]{6}$`], [Farbe im Bildfahrplan],
)

=== Formation <operations-Formation>

Eine Zugbildung: die Triebfahrzeuge über ihren Schlüssel im Katalog und ihre Prüfsumme, die Wagenzuglast, die Bremse.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [], [Text], [Name],
  [`tractionUnits`], [ja], [Liste von #link(<operations-TractionUnit>)[`TractionUnit`]], [die Triebfahrzeuge, von vorn nach hinten],
  [`hauledLoad`], [], [Objekt (Properties unten)], [die Wagenzuglast, als eine],
  [`hauledLoad.mass`], [], [Zahl; ≥ 0], [Masse \[kg\]],
  [`hauledLoad.length`], [], [Zahl; ≥ 0], [Länge \[m\]],
  [`hauledLoad.maxSpeed`], [], [Zahl; ≥ 0], [Höchstgeschwindigkeit \[m/s\]],
  [`brakePosition`], [], [eins von `G`, `P`, `R`, `R+Mg`], [Bremsstellung],
  [`brakedWeightPercentage`], [], [Zahl; ≥ 0; ≤ 300], [Bremshundertstel \[%\]],
)

=== TractionUnit <operations-TractionUnit>

Ein Triebfahrzeug einer Zugbildung.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`vehicle`], [ja], [Text], [Schlüssel des Fahrzeugs im Katalog der Suite],
  [`hash`], [], [Text; Muster `^sha256:[0-9a-f]{64}$`], [Prüfsumme des Fahrzeugs, mit dem der Zug gerechnet wurde; die Datei führt es mit (vehicles)],
)

=== Vehicle <operations-Vehicle>

Ein Fahrzeug, das eine Zugbildung nennt, mitgeführt: die Datei rechnet mit dem, was sie mitführt.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`key`], [ja], [Text], [Schlüssel im Katalog, aus dem es stammt],
  [`hash`], [ja], [Text; Muster `^sha256:[0-9a-f]{64}$`], [Prüfsumme des Fahrzeugs],
  [`rollingStock`], [ja], [Text; Muster `^sha256:[0-9a-f]{64}$`], [das Fahrzeug als RailJSON-Rollmaterial, als Blob — dieselbe Prüfsumme wie `hash`],
)

=== Connection <operations-Connection>

Ein Anschluss: von der Ankunft eines Zugs zur Abfahrt eines anderen an einem Ort.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`fromTrainId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.trains`], [der ankommende Zug],
  [`toTrainId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.trains`], [der abfahrende Zug],
  [`place`], [ja], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [der Ort],
  [`changeTime`], [], [Zahl; ≥ 0; ≤ 86400], [Übergangszeit \[s\]],
  [`waitUpTo`], [], [Zahl; ≥ 0; ≤ 86400], [wie lange der abfahrende Zug wartet \[s\]],
)

=== Turnaround <operations-Turnaround>

Eine Wende: vom Ende eines Zugs zu dem Zug, als der seine Fahrzeuge als Nächstes fahren.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`fromTrainId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.trains`], [der endende Zug],
  [`toTrainId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.trains`], [der folgende Zug],
  [`leastTime`], [], [Zahl; ≥ 0; ≤ 86400], [Mindestwendezeit \[s\]],
)

=== DelayScenario <operations-DelayScenario>

Ein Verspätungsszenario: jede Regel ein Anteil verspäteter Züge und eine Verteilung ihrer Verspätungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [ja], [Text], [Name],
  [`entryDelays`], [], [Liste von #link(<operations-DelayRule>)[`DelayRule`]], [Verspätungen beim Eintritt ins Planungsgebiet],
  [`dwellDelays`], [], [Liste von #link(<operations-DelayRule>)[`DelayRule`]], [verlängerte Haltezeiten],
)

=== DelayRule <operations-DelayRule>

Eine Regel eines Verspätungsszenarios.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`place`], [], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [wo sie gilt; ohne: überall],
  [`trainKindId`], [], [#link(<core-Ref>)[`Ref`] → `operations.trainKinds`], [für welche Zuggattung; ohne: für jede],
  [`share`], [ja], [Zahl; ≥ 0; ≤ 1], [Anteil der verspäteten Züge],
  [`distribution`], [ja], [#link(<operations-Distribution>)[`Distribution`]], [Verteilung der Verspätungen],
  [`source`], [], [Text], [woher die Zahlen stammen: Startwert der Suite oder eine Messung],
)

=== Distribution <operations-Distribution>

Eine Verteilung von Verspätungen.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`kind`], [ja], [eins der Werte unter Bedeutung], [Art der Verteilung #linebreak() `exponential` exponentiell (`mean`) #linebreak() `uniform` gleichverteilt (`min`, `max`) #linebreak() `empirical` empirisch (`values`)],
  [`mean`], [], [Zahl; ≥ 0], [Mittelwert \[s\], exponentiell],
  [`min`], [], [Zahl; ≥ 0], [kleinster Wert \[s\], gleichverteilt],
  [`max`], [], [Zahl; ≥ 0], [größter Wert \[s\], gleichverteilt],
  [`values`], [], [Liste von Zahl; ≥ 0], [Werte \[s\], empirisch: mit gleichem Gewicht gezogen],
)

=== Study <operations-Study>

Eine Kapazitätsuntersuchung: ein Fahrplan, ein Zeitraum, Abschnitte, Grenzwerte, ein Szenario, ihre Durchläufe und ihr Startwert.

_Weitere Properties sind nicht erlaubt._

#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },
  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],
  [`id`], [ja], [#link(<core-Id>)[`Id`]], [Kennung des Objekts],
  [`name`], [ja], [Text], [Name],
  [`timetableId`], [ja], [#link(<core-Ref>)[`Ref`] → `operations.timetables`], [der Fahrplan],
  [`period`], [], [Objekt (Properties unten)], [der untersuchte Zeitraum],
  [`period.from`], [ja], [Zahl; ≥ 0; ≤ 172799.999], [Anfang in Sekunden ab Mitternacht des Fahrplantags],
  [`period.to`], [ja], [Zahl; ≥ 0; ≤ 172799.999], [Ende in Sekunden ab Mitternacht des Fahrplantags],
  [`stretches`], [], [Liste von Objekten (Properties unten)], [die Abschnitte, von Ort zu Ort],
  [`stretches[].from`], [ja], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [von dieser Betriebsstelle],
  [`stretches[].to`], [ja], [#link(<core-Ref>)[`Ref`] → `core.operationalPoints`], [bis zu dieser],
  [`bounds`], [], [Objekt (Properties unten)], [die Grenzwerte, wo die Untersuchung sie ändert],
  [`bounds.occupancyRate`], [], [Zahl; ≥ 0; ≤ 1], [Grenze des Belegungsgrads (UIC-Merkblatt 406)],
  [`bounds.punctualityThreshold`], [], [Zahl; ≥ 0; ≤ 3600], [ab welcher Verspätung ein Zug nicht mehr pünktlich ist \[s\]],
  [`delayScenarioId`], [], [#link(<core-Ref>)[`Ref`] → `operations.delayScenarios`], [das Verspätungsszenario],
  [`runs`], [], [ganze Zahl; ≥ 1; ≤ 100000], [Anzahl der Durchläufe],
  [`seed`], [], [ganze Zahl; ≥ 0], [Startwert des Zufallsgenerators],
)
